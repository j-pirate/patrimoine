//! Independent persistence oracles for financing contracts on synthetic vaults.
use patrimoine_application::{DebtService, RegistryService, TreasuryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

fn fraction(num: &str, den: &str) -> Fraction {
    Fraction {
        num: num.into(),
        den: den.into(),
    }
}
fn entity(vault: &mut Vault, name: &str, kind: &str) -> Entity {
    vault
        .save_entity(SaveEntityInput {
            id: None,
            expected_revision_id: None,
            reason: "Contrepartie synthétique".into(),
            kind: kind.into(),
            display_name: name.into(),
            first_name: None,
            last_name: None,
            birth_date: None,
            residence_country: Some("FR".into()),
            preferred_currency: "EUR".into(),
            activity: String::new(),
            notes: String::new(),
            archived: false,
        })
        .unwrap()
}
fn contract_input(name: &str, currency: &str) -> SaveDebtContractInput {
    SaveDebtContractInput {
        id: None,
        expected_revision_id: None,
        reason: "Contrat synthétique".into(),
        name: name.into(),
        currency: currency.into(),
        tracking_start: "2024-01-01".into(),
        notes: "Aucune donnée réelle".into(),
        archived: false,
    }
}
struct Fixture {
    vault: Vault,
    creditor: Entity,
    debtor: Entity,
    other: Entity,
    contract: DebtContract,
    terms: DebtTerms,
}
fn fixture(root: &std::path::Path) -> Fixture {
    let mut vault = Vault::open(root).unwrap();
    let creditor = entity(&mut vault, "Créancier synthétique", "person");
    let debtor = entity(&mut vault, "Débiteur synthétique", "person");
    let other = entity(&mut vault, "Autre contrepartie synthétique", "person");
    let contract = vault
        .save_debt_contract(contract_input("Prêt synthétique", "EUR"))
        .unwrap();
    let terms = vault
        .save_debt_terms(SaveDebtTermsInput {
            id: None,
            expected_revision_id: None,
            reason: "Parties explicites du contrat synthétique".into(),
            debt_id: contract.id.clone(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            principal_status: "complete".into(),
            interest_rate: Some("0,0200".into()),
            interest_convention: "Taux annuel indicatif, aucun intérêt automatique".into(),
            parties: vec![
                FinancingParty {
                    line_id: "creditor".into(),
                    side: "creditor".into(),
                    entity_id: creditor.id.clone(),
                    legal_share: Some(fraction("1", "1")),
                    economic_share: Some(fraction("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
                FinancingParty {
                    line_id: "debtor".into(),
                    side: "debtor".into(),
                    entity_id: debtor.id.clone(),
                    legal_share: Some(fraction("1", "1")),
                    economic_share: Some(fraction("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
            ],
            notes: String::new(),
        })
        .unwrap();
    Fixture {
        vault,
        creditor,
        debtor,
        other,
        contract,
        terms,
    }
}
fn movement(
    contract: &DebtContract,
    kind: &str,
    on: &str,
    principal: &str,
    interest: &str,
    fees: &str,
) -> SaveDebtMovementInput {
    SaveDebtMovementInput {
        id: None,
        expected_revision_id: None,
        reason: "Mouvement synthétique motivé".into(),
        debt_id: contract.id.clone(),
        kind: kind.into(),
        effective_on: on.into(),
        principal_change: principal.into(),
        interest_paid: interest.into(),
        fees_paid: fees.into(),
        unlinked_reason: "Fait historique déclaré sans compte bancaire dans cette fixture".into(),
        notes: "Donnée synthétique".into(),
    }
}
fn report(f: &Fixture, on: &str, k: Option<u64>) -> DebtReport {
    f.vault
        .get_debt_report(DebtReportRequest {
            debt_id: f.contract.id.clone(),
            date: on.into(),
            knowledge_sequence: k,
        })
        .unwrap()
}
fn state(f: &Fixture) -> Value {
    serde_json::to_value(f.vault.list_debts_at(None).unwrap()).unwrap()
}
fn opening(f: &mut Fixture, amount: &str) -> DebtMovement {
    f.vault
        .save_debt_movement(movement(
            &f.contract,
            "opening",
            "2024-01-01",
            amount,
            "0",
            "0",
        ))
        .unwrap()
}

#[test]
fn missing_opening_is_unknown_and_explicit_zero_does_not_become_missing() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    assert_eq!(report(&f, "2024-02-01", None).balance.exact_principal, None);
    assert_eq!(f.terms.interest_rate.as_deref(), Some("0.02"));
    let before = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    opening(&mut f, "0,000");
    assert_eq!(
        report(&f, "2024-02-01", None).balance.exact_principal,
        Some(fraction("0", "1"))
    );
    assert_eq!(
        report(&f, "2024-02-01", Some(before))
            .balance
            .exact_principal,
        None
    );
}

#[test]
fn repayment_separates_principal_interest_fees_and_preserves_decimal_sources() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "100000");
    let old_k = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    let posted = f
        .vault
        .save_debt_movement(movement(
            &f.contract,
            "principal_repayment",
            "2024-02-01",
            "-1000,00",
            "200,0",
            "10,00",
        ))
        .unwrap();
    assert_eq!(
        (
            &posted.principal_change,
            &posted.interest_paid,
            &posted.fees_paid
        ),
        (&"-1000".into(), &"200".into(), &"10".into())
    );
    let r = report(&f, "2024-02-01", None);
    assert_eq!(r.balance.exact_principal, Some(fraction("99000", "1")));
    assert_eq!(r.balance.exact_interest_paid, fraction("200", "1"));
    assert_eq!(r.balance.exact_fees_paid, fraction("10", "1"));
    assert_eq!(
        report(&f, "2024-02-01", Some(old_k))
            .balance
            .exact_principal,
        Some(fraction("100000", "1"))
    );
    assert!(r.source_ids.contains(&posted.revision_id));
}

#[test]
fn overpayment_and_retroactive_opening_correction_leave_no_partial_change() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    let initial = opening(&mut f, "100000");
    f.vault
        .save_debt_movement(movement(
            &f.contract,
            "principal_repayment",
            "2024-02-01",
            "-1000",
            "0",
            "0",
        ))
        .unwrap();
    let before = state(&f);
    assert!(f
        .vault
        .save_debt_movement(movement(
            &f.contract,
            "principal_repayment",
            "2024-03-01",
            "-100000",
            "0",
            "0"
        ))
        .is_err());
    assert_eq!(state(&f), before);
    let mut correction = movement(&f.contract, "opening", "2024-01-01", "500", "0", "0");
    correction.id = Some(initial.id);
    correction.expected_revision_id = Some(initial.revision_id);
    assert!(f.vault.save_debt_movement(correction).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn one_opening_and_immutable_contract_currency_and_start_are_enforced() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "100");
    let before = state(&f);
    assert!(f
        .vault
        .save_debt_movement(movement(
            &f.contract,
            "opening",
            "2024-01-01",
            "100",
            "0",
            "0"
        ))
        .is_err());
    assert_eq!(state(&f), before);
    let mut edit = contract_input("Devise changée", "USD");
    edit.id = Some(f.contract.id.clone());
    edit.expected_revision_id = Some(f.contract.revision_id.clone());
    assert!(f.vault.save_debt_contract(edit.clone()).is_err());
    assert_eq!(state(&f), before);
    edit.currency = "EUR".into();
    edit.tracking_start = "2023-01-01".into();
    assert!(f.vault.save_debt_contract(edit).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn obsolete_revision_and_future_knowledge_are_rejected_without_mutation() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    let old = opening(&mut f, "1000");
    let mut edit = movement(&f.contract, "opening", "2024-01-01", "2000", "0", "0");
    edit.id = Some(old.id);
    edit.expected_revision_id = Some(old.revision_id);
    f.vault.save_debt_movement(edit.clone()).unwrap();
    let before = state(&f);
    assert!(f.vault.save_debt_movement(edit).is_err());
    assert_eq!(state(&f), before);
    let k = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    assert!(f.vault.list_debts_at(Some(k + 1)).is_err());
    assert!(f
        .vault
        .get_debt_report(DebtReportRequest {
            debt_id: f.contract.id.clone(),
            date: "2024-01-01".into(),
            knowledge_sequence: Some(k + 1)
        })
        .is_err());
}

#[test]
fn archiving_a_contract_keeps_its_certain_balance_and_previous_profile() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "1000");
    let old_k = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    let mut edit = contract_input("Contrat archivé", "EUR");
    edit.id = Some(f.contract.id.clone());
    edit.expected_revision_id = Some(f.contract.revision_id.clone());
    edit.archived = true;
    f.vault.save_debt_contract(edit).unwrap();
    let current = report(&f, "2024-03-01", None);
    assert!(current.contract.archived);
    assert_eq!(current.balance.exact_principal, Some(fraction("1000", "1")));
    let historical = report(&f, "2024-03-01", Some(old_k));
    assert!(!historical.contract.archived);
    assert_eq!(historical.contract.name, "Prêt synthétique");
}

#[test]
fn party_fractions_normalize_but_duplicates_and_excess_are_atomic_rejections() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    let mut edit = SaveDebtTermsInput {
        id: Some(f.terms.id.clone()),
        expected_revision_id: Some(f.terms.revision_id.clone()),
        reason: "Correction de répartition synthétique".into(),
        debt_id: f.contract.id.clone(),
        valid_from: f.terms.valid_from.clone(),
        valid_to: None,
        principal_status: "complete".into(),
        interest_rate: None,
        interest_convention: String::new(),
        parties: f.terms.parties.clone(),
        notes: String::new(),
    };
    edit.parties[0].economic_share = Some(fraction("50", "100"));
    let added = FinancingParty {
        line_id: "other-creditor".into(),
        side: "creditor".into(),
        entity_id: f.other.id.clone(),
        legal_share: None,
        economic_share: Some(fraction("1", "2")),
        liability_kind: "several".into(),
        notes: String::new(),
    };
    edit.parties.push(added);
    let valid = f.vault.save_debt_terms(edit.clone()).unwrap();
    assert_eq!(
        valid
            .parties
            .iter()
            .find(|p| p.entity_id == f.creditor.id)
            .unwrap()
            .economic_share,
        Some(fraction("1", "2"))
    );
    edit.expected_revision_id = Some(valid.revision_id);
    edit.parties[0].economic_share = Some(fraction("3", "4"));
    let before = state(&f);
    assert!(f.vault.save_debt_terms(edit.clone()).is_err());
    assert_eq!(state(&f), before);
    edit.parties[0].economic_share = Some(fraction("1", "2"));
    edit.parties[2].entity_id = f.creditor.id.clone();
    assert!(f.vault.save_debt_terms(edit).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn schedules_are_forecasts_and_partial_allocations_keep_the_late_settlement() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "100000");
    let schedule = f
        .vault
        .save_debt_schedule(SaveDebtScheduleInput {
            id: None,
            expected_revision_id: None,
            reason: "Échéancier synthétique".into(),
            debt_id: f.contract.id.clone(),
            due_on: "2024-02-05".into(),
            expected_principal: "1000".into(),
            expected_interest: "200".into(),
            expected_fees: "10".into(),
            status: "planned".into(),
            notes: String::new(),
        })
        .unwrap();
    assert_eq!(
        report(&f, "2024-02-06", None).balance.exact_principal,
        Some(fraction("100000", "1"))
    );
    let repayment = f
        .vault
        .save_debt_movement(movement(
            &f.contract,
            "principal_repayment",
            "2024-02-20",
            "-1000",
            "200",
            "10",
        ))
        .unwrap();
    f.vault
        .save_debt_payment_allocation(SaveDebtPaymentAllocationInput {
            id: None,
            expected_revision_id: None,
            reason: "Affectation synthétique".into(),
            movement_id: repayment.id.clone(),
            schedule_id: schedule.id.clone(),
            effective_on: "2024-02-20".into(),
            principal: "1000".into(),
            interest: "200".into(),
            fees: "10".into(),
        })
        .unwrap();
    let r = report(&f, "2024-02-28", None);
    assert_eq!(r.balance.exact_principal, Some(fraction("99000", "1")));
    assert!(r.schedules[0].historical_late);
    assert_eq!(r.schedules[0].settled_on.as_deref(), Some("2024-02-20"));
    let before = state(&f);
    assert!(f
        .vault
        .save_debt_payment_allocation(SaveDebtPaymentAllocationInput {
            id: None,
            expected_revision_id: None,
            reason: "Dépassement refusé".into(),
            movement_id: repayment.id,
            schedule_id: schedule.id,
            effective_on: "2024-02-21".into(),
            principal: "1".into(),
            interest: "0".into(),
            fees: "0".into()
        })
        .is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn receivable_impairment_does_not_reduce_the_borrowers_nominal_and_cannot_exceed_it() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "1000");
    f.vault
        .save_debt_receivable_adjustment(SaveDebtReceivableAdjustmentInput {
            id: None,
            expected_revision_id: None,
            reason: "Dépréciation synthétique".into(),
            debt_id: f.contract.id.clone(),
            creditor_entity_id: f.creditor.id.clone(),
            kind: "impairment".into(),
            effective_on: "2024-02-01".into(),
            valuation_delta: "-200".into(),
            source: "Estimation personnelle synthétique".into(),
            notes: String::new(),
        })
        .unwrap();
    let r = report(&f, "2024-02-01", None);
    assert_eq!(r.balance.exact_principal, Some(fraction("1000", "1")));
    let creditor = r
        .parties
        .iter()
        .find(|p| p.party.side == "creditor")
        .unwrap();
    let debtor = r.parties.iter().find(|p| p.party.side == "debtor").unwrap();
    assert_eq!(creditor.exact_carrying_value, Some(fraction("800", "1")));
    assert_eq!(debtor.exact_carrying_value, Some(fraction("1000", "1")));
    let before = state(&f);
    assert!(f
        .vault
        .save_debt_movement(movement(
            &f.contract,
            "principal_repayment",
            "2024-03-01",
            "-900",
            "0",
            "0"
        ))
        .is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn guarantees_remain_contingent_and_collateral_does_not_assign_the_debt() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "1000");
    let asset = f
        .vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Sûreté synthétique".into(),
            name: "Bien garanti".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    f.vault
        .save_debt_asset_link(SaveDebtAssetLinkInput {
            id: None,
            expected_revision_id: None,
            reason: "Lien de sûreté synthétique".into(),
            debt_id: f.contract.id.clone(),
            asset_id: asset.id,
            relation_kind: "collateral".into(),
            attributed_principal_share: None,
            valid_from: "2024-01-01".into(),
            valid_to: None,
            notes: String::new(),
        })
        .unwrap();
    f.vault
        .save_debt_guarantee(SaveDebtGuaranteeInput {
            id: None,
            expected_revision_id: None,
            reason: "Garantie synthétique".into(),
            debt_id: f.contract.id.clone(),
            guarantor_entity_id: f.other.id.clone(),
            limit_amount: Some("50000".into()),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            notes: String::new(),
        })
        .unwrap();
    let r = report(&f, "2024-02-01", None);
    assert_eq!(r.balance.exact_principal, Some(fraction("1000", "1")));
    assert_eq!(r.guarantees.len(), 1);
    assert!(r.warnings.iter().any(|w| w.code == "contingent_guarantee"));
    assert_eq!(
        r.parties
            .iter()
            .find(|p| p.party.side == "debtor")
            .unwrap()
            .party
            .entity_id,
        f.debtor.id
    );
}

#[test]
fn voided_terms_copy_their_parties_and_keep_old_knowledge_after_reopening() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "1000");
    let old_k = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    f.vault
        .void_registry_record(VoidRecordInput {
            id: f.terms.id.clone(),
            expected_revision_id: f.terms.revision_id.clone(),
            reason: "Annulation synthétique de conditions".into(),
        })
        .unwrap();
    assert!(report(&f, "2024-02-01", None).terms.is_none());
    assert_eq!(
        report(&f, "2024-02-01", Some(old_k))
            .terms
            .unwrap()
            .parties
            .len(),
        2
    );
    drop(f.vault);
    let reopened = Vault::open(temp.path()).unwrap();
    assert!(reopened.list_debts_at(None).unwrap().terms.is_empty());
    assert_eq!(
        reopened.list_debts_at(Some(old_k)).unwrap().terms[0]
            .parties
            .len(),
        2
    );
}

#[test]
fn backup_restore_preserves_contract_parties_movements_adjustments_and_old_results() {
    let temp = TempDir::new().unwrap();
    let source = temp.path().join("source");
    let mut f = fixture(&source);
    opening(&mut f, "100000");
    let old_k = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    f.vault
        .save_debt_movement(movement(
            &f.contract,
            "principal_repayment",
            "2024-02-01",
            "-1000",
            "200",
            "10",
        ))
        .unwrap();
    f.vault
        .save_debt_receivable_adjustment(SaveDebtReceivableAdjustmentInput {
            id: None,
            expected_revision_id: None,
            reason: "Dépréciation synthétique à sauvegarder".into(),
            debt_id: f.contract.id.clone(),
            creditor_entity_id: f.creditor.id.clone(),
            kind: "impairment".into(),
            effective_on: "2024-02-15".into(),
            valuation_delta: "-100".into(),
            source: "Estimation de test, sans remise au débiteur".into(),
            notes: String::new(),
        })
        .unwrap();
    let before = state(&f);
    let archive = temp.path().join("financements.zip");
    f.vault.create_backup(&archive).unwrap();
    let mut restored = Vault::open(temp.path().join("restored")).unwrap();
    let inspection = restored.inspect_restore(&archive).unwrap();
    restored
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(
        serde_json::to_value(restored.list_debts_at(None).unwrap()).unwrap(),
        before
    );
    assert_eq!(
        restored
            .get_debt_report(DebtReportRequest {
                debt_id: f.contract.id.clone(),
                date: "2024-03-01".into(),
                knowledge_sequence: Some(old_k)
            })
            .unwrap()
            .balance
            .exact_principal,
        Some(fraction("100000", "1"))
    );
    assert_eq!(
        restored
            .get_debt_report(DebtReportRequest {
                debt_id: f.contract.id,
                date: "2024-03-01".into(),
                knowledge_sequence: None
            })
            .unwrap()
            .balance
            .exact_principal,
        Some(fraction("99000", "1"))
    );
}

#[test]
fn atomic_repayment_correction_preserves_one_cash_origin_and_exact_current_revisions() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(temp.path());
    opening(&mut f, "100000");
    let asset = f
        .vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Compte de remboursement synthétique".into(),
            name: "Compte débiteur".into(),
            country: "FR".into(),
            category: "other".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    let account = f
        .vault
        .save_treasury_account(SaveTreasuryAccountInput {
            id: None,
            expected_revision_id: None,
            reason: "Titulaire débiteur explicitement identifié".into(),
            asset_id: asset.id,
            kind: "bank".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            legal_owner_entity_id: f.debtor.id.clone(),
            bank_name: "Banque synthétique".into(),
            reference: "COMPTE-TEST".into(),
            country: "FR".into(),
            storage_location: String::new(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    let cash = |kind: &str, on: &str, amount: &str, line: &str| SaveTreasuryTransactionInput {
        id: None,
        expected_revision_id: None,
        reason: "Écriture de remboursement synthétique".into(),
        kind: kind.into(),
        effective_on: on.into(),
        label: "Test de remboursement indivisible".into(),
        reference: String::new(),
        notes: String::new(),
        entries: vec![TreasuryEntry {
            line_id: line.into(),
            account_id: account.id.clone(),
            amount: amount.into(),
            role: if kind == "opening" {
                "opening"
            } else {
                "principal"
            }
            .into(),
        }],
    };
    f.vault
        .save_treasury_transaction(cash(
            "opening",
            "2024-01-01",
            "200000",
            &Uuid::new_v4().to_string(),
        ))
        .unwrap();
    let line = Uuid::new_v4().to_string();
    let links = |principal: &str, interest: &str, fee: &str| {
        [
            ("principal", principal),
            ("interest", interest),
            ("fee", fee),
        ]
        .into_iter()
        .map(|(component, amount)| PostedDebtCashLink {
            line_id: line.clone(),
            side: "debtor".into(),
            component: component.into(),
            amount: amount.into(),
        })
        .collect::<Vec<_>>()
    };
    let state = |f: &Fixture| {
        serde_json::json!({
            "debts": f.vault.list_debts_at(None).unwrap(),
            "treasury": f.vault.list_treasury_at(None).unwrap(),
            "history": f.vault.list_registry_history().unwrap(),
        })
    };
    let before = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    let posted = f
        .vault
        .post_debt_movement(PostDebtMovementInput {
            expected_sequence: before,
            movement: movement(
                &f.contract,
                "principal_repayment",
                "2024-02-01",
                "-1000",
                "200",
                "10",
            ),
            transaction: Some(cash("loan", "2024-02-01", "-1210", &line)),
            existing_transaction_revision_id: None,
            links: links("-1000", "-200", "-10"),
        })
        .unwrap();
    let posted_k = before + 1;
    let treasury = f.vault.list_treasury_at(None).unwrap();
    assert_eq!(treasury.knowledge_sequence, posted_k);
    assert_eq!(treasury.transactions.len(), 2);
    assert_eq!(treasury.cash_links.len(), 3);
    assert!(treasury
        .cash_links
        .iter()
        .all(|link| link.source_revision_id == posted.revision_id));
    let ledger = treasury
        .transactions
        .iter()
        .find(|t| t.kind == "loan")
        .unwrap()
        .clone();
    assert_eq!(
        report(&f, "2024-02-01", None).balance.exact_principal,
        Some(fraction("99000", "1"))
    );
    assert_eq!(
        f.vault
            .get_treasury_report(TreasuryReportRequest {
                account_id: account.id.clone(),
                date: "2024-02-01".into(),
                knowledge_sequence: None,
            })
            .unwrap()
            .balance_original
            .as_deref(),
        Some("198790.00")
    );
    let old_state = state(&f);
    let mut correction = movement(
        &f.contract,
        "principal_repayment",
        "2024-02-01",
        "-1500",
        "210",
        "15",
    );
    correction.id = Some(posted.id.clone());
    correction.expected_revision_id = Some(posted.revision_id.clone());
    assert!(f.vault.save_debt_movement(correction.clone()).is_err());
    assert_eq!(state(&f), old_state);
    let mut command = PostDebtMovementInput {
        expected_sequence: posted_k,
        movement: correction.clone(),
        transaction: Some(cash("loan", "2024-02-01", "-1725", &line)),
        existing_transaction_revision_id: None,
        links: links("-1500", "-210", "-15"),
    };
    assert!(f.vault.post_debt_movement(command.clone()).is_err());
    assert_eq!(state(&f), old_state);
    let other = f
        .vault
        .save_treasury_transaction(cash(
            "loan",
            "2024-02-01",
            "-1725",
            &Uuid::new_v4().to_string(),
        ))
        .unwrap();
    let before_correction = state(&f);
    command.expected_sequence = f.vault.list_debts_at(None).unwrap().knowledge_sequence;
    let mut other_command = command.clone();
    other_command.transaction = None;
    other_command.existing_transaction_revision_id = Some(other.revision_id);
    for link in &mut other_command.links {
        link.line_id = other.entries[0].line_id.clone();
    }
    assert!(f.vault.post_debt_movement(other_command).is_err());
    assert_eq!(state(&f), before_correction);
    command.transaction.as_mut().unwrap().id = Some(ledger.id.clone());
    command.transaction.as_mut().unwrap().expected_revision_id = Some(ledger.revision_id.clone());
    let mut stale_sequence = command.clone();
    stale_sequence.expected_sequence = posted_k;
    assert!(f.vault.post_debt_movement(stale_sequence).is_err());
    assert_eq!(state(&f), before_correction);
    let current_k = command.expected_sequence + 1;
    let corrected = f.vault.post_debt_movement(command.clone()).unwrap();
    assert_eq!(corrected.id, posted.id);
    assert_ne!(corrected.revision_id, posted.revision_id);
    let current = f.vault.list_treasury_at(None).unwrap();
    assert_eq!(current.knowledge_sequence, current_k);
    assert_eq!(current.transactions.len(), 3);
    assert_eq!(current.cash_links.len(), 3);
    let current_ledger = current
        .transactions
        .iter()
        .find(|t| t.id == ledger.id)
        .unwrap();
    assert_ne!(current_ledger.revision_id, ledger.revision_id);
    assert!(current
        .cash_links
        .iter()
        .all(|link| link.source_revision_id == corrected.revision_id
            && link.transaction_revision_id == current_ledger.revision_id));
    let balance = report(&f, "2024-02-01", None).balance;
    assert_eq!(balance.exact_principal, Some(fraction("98500", "1")));
    assert_eq!(balance.exact_interest_paid, fraction("210", "1"));
    assert_eq!(balance.exact_fees_paid, fraction("15", "1"));
    assert_eq!(
        f.vault
            .get_treasury_report(TreasuryReportRequest {
                account_id: account.id,
                date: "2024-02-01".into(),
                knowledge_sequence: None,
            })
            .unwrap()
            .balance_original
            .as_deref(),
        Some("196550.00")
    );
    let unchanged = state(&f);
    command.expected_sequence = current_k;
    command.transaction = None;
    command.existing_transaction_revision_id = Some(current_ledger.revision_id.clone());
    assert!(f.vault.post_debt_movement(command.clone()).is_err());
    assert_eq!(state(&f), unchanged);
    command.movement.expected_revision_id = Some(corrected.revision_id);
    command.existing_transaction_revision_id = Some(ledger.revision_id);
    assert!(f.vault.post_debt_movement(command).is_err());
    assert_eq!(state(&f), unchanged);
    assert_eq!(
        serde_json::to_value(f.vault.list_debts_at(Some(posted_k)).unwrap()).unwrap(),
        old_state["debts"]
    );
    assert_eq!(
        serde_json::to_value(f.vault.list_treasury_at(Some(posted_k)).unwrap()).unwrap(),
        old_state["treasury"]
    );
}
