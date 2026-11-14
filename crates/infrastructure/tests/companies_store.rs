//! Independent SQLite and journal oracles for company management/distributions.
use patrimoine_application::{
    CompanyService, MetalService, RegistryService, TreasuryService, VaultService,
};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
use tempfile::TempDir;

fn entity(v: &mut Vault, name: &str, kind: &str) -> Entity {
    v.save_entity(SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Identité synthétique".into(),
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
fn asset(v: &mut Vault, owner: &Entity, name: &str) -> Asset {
    let a = v
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Actif synthétique".into(),
            name: name.into(),
            country: "FR".into(),
            category: "other".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    v.save_ownership(SaveOwnershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Droit synthétique".into(),
        asset_id: a.id.clone(),
        owner_entity_id: owner.id.clone(),
        legal_share: Some(Fraction {
            num: "1".into(),
            den: "1".into(),
        }),
        economic_share: Some(Fraction {
            num: "1".into(),
            den: "1".into(),
        }),
        valid_from: "2025-01-01".into(),
        valid_to: None,
    })
    .unwrap();
    a
}
fn account(v: &mut Vault, owner: &Entity, amount: &str) -> TreasuryAccount {
    let a = asset(v, owner, "Compte du test société");
    let a = v
        .save_treasury_account(SaveTreasuryAccountInput {
            id: None,
            expected_revision_id: None,
            reason: "Compte synthétique".into(),
            asset_id: a.id,
            kind: "bank".into(),
            currency: "EUR".into(),
            tracking_start: "2025-01-01".into(),
            legal_owner_entity_id: owner.id.clone(),
            bank_name: "Banque synthétique".into(),
            reference: String::new(),
            country: "FR".into(),
            storage_location: String::new(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    v.save_treasury_transaction(SaveTreasuryTransactionInput {
        id: None,
        expected_revision_id: None,
        reason: "Ouverture confirmée".into(),
        kind: "opening".into(),
        effective_on: "2025-01-01".into(),
        label: "Stock explicite".into(),
        reference: String::new(),
        notes: String::new(),
        entries: vec![TreasuryEntry {
            line_id: "opening".into(),
            account_id: a.id.clone(),
            amount: amount.into(),
            role: "opening".into(),
        }],
    })
    .unwrap();
    a
}
struct Fixture {
    vault: Vault,
    company: Entity,
    beneficiary: Entity,
    payer: TreasuryAccount,
    recipient: TreasuryAccount,
}
fn fixture(root: &std::path::Path) -> Fixture {
    let mut vault = Vault::open(root).unwrap();
    let company = entity(&mut vault, "Société synthétique", "company");
    let beneficiary = entity(&mut vault, "Associé synthétique", "person");
    vault
        .save_company_policy(SaveCompanyPolicyInput {
            id: None,
            expected_revision_id: None,
            reason: "Méthode détaillée explicite".into(),
            company_id: company.id.clone(),
            mode: "detailed".into(),
            coverage_complete: true,
            liability_qualified: true,
            valid_from: "2025-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    let payer = account(&mut vault, &company, "10000");
    let recipient = account(&mut vault, &beneficiary, "0");
    Fixture {
        vault,
        company,
        beneficiary,
        payer,
        recipient,
    }
}
fn period(
    f: &Fixture,
    start: &str,
    end: &str,
    frequency: &str,
    currency: &str,
) -> SaveCompanyFinancialPeriodInput {
    SaveCompanyFinancialPeriodInput {
        id: None,
        expected_revision_id: None,
        reason: "Observation périodique synthétique".into(),
        company_id: f.company.id.clone(),
        period_start: start.into(),
        period_end: end.into(),
        frequency: frequency.into(),
        currency: currency.into(),
        revenue: None,
        operating_expenses: None,
        operating_result: None,
        net_result: None,
        indicative_cash: None,
        indicative_debt: None,
        important_receivables: None,
        dividends_declared: None,
        indicative_global_value: None,
        notes: String::new(),
    }
}
fn distribution(
    f: &Fixture,
    kind: &str,
    amount: &str,
    parent: Option<&CompanyDistribution>,
) -> SaveCompanyDistributionInput {
    SaveCompanyDistributionInput {
        id: None,
        expected_revision_id: None,
        reason: "Distribution synthétique".into(),
        company_id: f.company.id.clone(),
        beneficiary_entity_id: f.beneficiary.id.clone(),
        kind: kind.into(),
        effective_on: if kind == "declared" {
            "2025-03-01"
        } else {
            "2025-03-15"
        }
        .into(),
        amount: amount.into(),
        currency: "EUR".into(),
        related_declaration_id: parent.map(|p| p.id.clone()),
        unlinked_reason: String::new(),
        notes: String::new(),
        cash_links: vec![],
    }
}
fn journal(f: &Fixture, amount: &str) -> SaveTreasuryTransactionInput {
    SaveTreasuryTransactionInput {
        id: None,
        expected_revision_id: None,
        reason: "Deux faces de trésorerie réelles".into(),
        kind: "dividend".into(),
        effective_on: "2025-03-15".into(),
        label: "Paiement de dividende".into(),
        reference: "DIV-TEST".into(),
        notes: String::new(),
        entries: vec![
            TreasuryEntry {
                line_id: "payer".into(),
                account_id: f.payer.id.clone(),
                amount: format!("-{amount}"),
                role: "principal".into(),
            },
            TreasuryEntry {
                line_id: "beneficiary".into(),
                account_id: f.recipient.id.clone(),
                amount: amount.into(),
                role: "principal".into(),
            },
        ],
    }
}
fn links(amount: &str) -> Vec<PostedCompanyDistributionCashLink> {
    vec![
        PostedCompanyDistributionCashLink {
            side: "payer".into(),
            line_id: "payer".into(),
            amount: format!("-{amount}"),
        },
        PostedCompanyDistributionCashLink {
            side: "beneficiary".into(),
            line_id: "beneficiary".into(),
            amount: amount.into(),
        },
    ]
}
fn posting(
    f: &Fixture,
    amount: &str,
    parent: Option<&CompanyDistribution>,
) -> PostCompanyDistributionInput {
    PostCompanyDistributionInput {
        expected_sequence: f.vault.status().unwrap().mutation_sequence,
        distribution: distribution(f, "paid", amount, parent),
        transaction: Some(journal(f, amount)),
        existing_transaction_revision_id: None,
        links: links(amount),
    }
}
fn corrected_posting(
    f: &Fixture,
    old: &CompanyDistribution,
    ledger: &TreasuryTransaction,
    amount: &str,
) -> PostCompanyDistributionInput {
    let mut input = posting(f, amount, None);
    input.distribution.id = Some(old.id.clone());
    input.distribution.expected_revision_id = Some(old.revision_id.clone());
    input.distribution.related_declaration_id = old.related_declaration_id.clone();
    let transaction = input.transaction.as_mut().unwrap();
    transaction.id = Some(ledger.id.clone());
    transaction.expected_revision_id = Some(ledger.revision_id.clone());
    input
}
fn ledger(f: &Fixture, distribution: &CompanyDistribution) -> TreasuryTransaction {
    let revision = &distribution.cash_links[0].transaction_revision_id;
    f.vault
        .list_treasury_at(None)
        .unwrap()
        .transactions
        .into_iter()
        .find(|t| &t.revision_id == revision)
        .unwrap()
}
fn balance(v: &Vault, account: &TreasuryAccount, k: Option<u64>) -> String {
    v.get_treasury_report(TreasuryReportRequest {
        account_id: account.id.clone(),
        date: "2025-12-31".into(),
        knowledge_sequence: k,
    })
    .unwrap()
    .balance_original
    .unwrap()
}
fn report(f: &Fixture, k: Option<u64>) -> CompanyReport {
    f.vault
        .get_company_report(CompanyReportRequest {
            company_id: f.company.id.clone(),
            date: "2025-12-31".into(),
            frequency: None,
            knowledge_sequence: k,
        })
        .unwrap()
}
fn state(v: &Vault) -> Value {
    json!({"companies":v.list_companies_at(None).unwrap(),"treasury":v.list_treasury_at(None).unwrap(),"metals":v.list_metals_at(None).unwrap(),"registry":v.list_registry().unwrap(),"history":v.list_registry_history().unwrap()})
}
fn connection(v: &Vault) -> Connection {
    let root = PathBuf::from(v.status().unwrap().data_dir);
    let generation = fs::read_to_string(root.join("CURRENT")).unwrap();
    Connection::open(
        root.join("generations")
            .join(generation.trim())
            .join("patrimoine.sqlite3"),
    )
    .unwrap()
}

#[test]
fn nullable_observations_and_parallel_frequencies_do_not_create_cash_or_duplicate_a_period() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let original_cash = serde_json::to_value(f.vault.list_treasury_at(None).unwrap()).unwrap();
    let mut january = period(&f, "2025-01-01", "2025-02-01", "monthly", "EUR");
    january.revenue = Some("100,00".into());
    january.operating_expenses = Some("20".into());
    january.indicative_cash = Some("50000".into());
    january.indicative_debt = Some("30000".into());
    january.indicative_global_value = Some("900000".into());
    let january = f.vault.save_company_financial_period(january).unwrap();
    let old_k = f.vault.status().unwrap().mutation_sequence;
    let mut february = period(&f, "2025-02-01", "2025-03-01", "monthly", "EUR");
    february.revenue = Some("0".into());
    f.vault.save_company_financial_period(february).unwrap();
    let annual = period(&f, "2025-01-01", "2026-01-01", "annual", "USD");
    f.vault.save_company_financial_period(annual).unwrap();
    let current = report(&f, None);
    assert_eq!(current.financial_periods.len(), 3);
    let jan = current
        .financial_periods
        .iter()
        .find(|p| p.period.id == january.id)
        .unwrap();
    assert_eq!(jan.period.revenue.as_deref(), Some("100"));
    assert_eq!(jan.period.net_result, None);
    assert_eq!(jan.calculated_operating_result.as_deref(), Some("80.00"));
    let feb = current
        .financial_periods
        .iter()
        .find(|p| p.period.period_start == "2025-02-01")
        .unwrap();
    assert_eq!(feb.period.revenue.as_deref(), Some("0"));
    assert_eq!(feb.period.net_result, None);
    assert_eq!(
        feb.trends
            .iter()
            .find(|t| t.metric == "revenue")
            .unwrap()
            .difference_original,
        "-100.00"
    );
    assert!(current
        .warnings
        .iter()
        .any(|w| w.code == "mixed_company_frequencies"));
    assert_eq!(
        current.accounts[0].report.balance_original.as_deref(),
        Some("10000.00")
    );
    assert!(current.debt_positions.is_empty());
    let before = state(&f.vault);
    let duplicate = period(&f, "2025-01-01", "2025-02-01", "monthly", "USD");
    assert!(f.vault.save_company_financial_period(duplicate).is_err());
    assert_eq!(state(&f.vault), before);
    let mut original_cash = original_cash;
    let mut current_cash = serde_json::to_value(f.vault.list_treasury_at(None).unwrap()).unwrap();
    original_cash
        .as_object_mut()
        .unwrap()
        .remove("knowledge_sequence");
    current_cash
        .as_object_mut()
        .unwrap()
        .remove("knowledge_sequence");
    assert_eq!(current_cash, original_cash);
    assert_eq!(
        f.vault
            .list_companies_at(Some(old_k))
            .unwrap()
            .financial_periods
            .len(),
        1
    );
    assert_eq!(balance(&f.vault, &f.payer, None), "10000.00");
    let sql = connection(&f.vault);
    assert_eq!(
        sql.query_row(
            "SELECT count(*) FROM company_financial_periods WHERE net_result IS NULL",
            [],
            |r| r.get::<_, u64>(0)
        )
        .unwrap(),
        3
    );
}

#[test]
fn declared_dividend_is_not_a_payment_and_actual_faces_share_one_atomic_change() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let declaration = distribution(&f, "declared", "1000", None);
    let declaration = f.vault.save_company_distribution(declaration).unwrap();
    assert_eq!(balance(&f.vault, &f.payer, None), "10000.00");
    assert_eq!(balance(&f.vault, &f.recipient, None), "0.00");
    let old_k = f.vault.status().unwrap().mutation_sequence;
    let input = posting(&f, "400", Some(&declaration));
    let paid = f.vault.post_company_distribution(input).unwrap();
    assert_eq!(f.vault.status().unwrap().mutation_sequence, old_k + 1);
    assert_eq!(paid.cash_links.len(), 2);
    assert_eq!(balance(&f.vault, &f.payer, None), "9600.00");
    assert_eq!(balance(&f.vault, &f.recipient, None), "400.00");
    assert_eq!(balance(&f.vault, &f.payer, Some(old_k)), "10000.00");
    let current = report(&f, None);
    let declared = current
        .distributions
        .iter()
        .find(|d| d.distribution.id == declaration.id)
        .unwrap();
    assert_eq!(declared.paid_against_declaration.as_deref(), Some("400.00"));
    assert_eq!(declared.remaining_declared.as_deref(), Some("600.00"));
    let payment = current
        .distributions
        .iter()
        .find(|d| d.distribution.id == paid.id)
        .unwrap();
    assert!(payment.payer_cash_complete && payment.beneficiary_cash_complete);
    let ledger = ledger(&f, &paid);
    let sql = connection(&f.vault);
    let changes = sql
        .query_row(
            "SELECT count(DISTINCT change_set_id) FROM revisions WHERE id IN(?1,?2)",
            [&paid.revision_id, &ledger.revision_id],
            |r| r.get::<_, u64>(0),
        )
        .unwrap();
    assert_eq!(changes, 1);
    assert_eq!(
        f.vault.list_treasury_at(None).unwrap().transactions.len(),
        3
    );
}

#[test]
fn atomic_dividend_correction_keeps_one_bank_origin_and_rejects_every_stale_selector() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let input = posting(&f, "400", None);
    let old = f.vault.post_company_distribution(input).unwrap();
    let old_ledger = ledger(&f, &old);
    let old_k = f.vault.status().unwrap().mutation_sequence;
    let input = corrected_posting(&f, &old, &old_ledger, "600");
    let corrected = f.vault.post_company_distribution(input).unwrap();
    let new_ledger = ledger(&f, &corrected);
    assert_eq!(corrected.id, old.id);
    assert_ne!(corrected.revision_id, old.revision_id);
    assert_eq!(new_ledger.id, old_ledger.id);
    assert_ne!(new_ledger.revision_id, old_ledger.revision_id);
    assert_eq!(f.vault.status().unwrap().mutation_sequence, old_k + 1);
    assert_eq!(
        f.vault.list_treasury_at(None).unwrap().transactions.len(),
        3
    );
    assert_eq!(balance(&f.vault, &f.payer, None), "9400.00");
    assert_eq!(balance(&f.vault, &f.recipient, None), "600.00");
    assert_eq!(balance(&f.vault, &f.payer, Some(old_k)), "9600.00");
    let old_snapshot = f.vault.list_companies_at(Some(old_k)).unwrap();
    assert_eq!(old_snapshot.distributions[0], old);
    let before = state(&f.vault);
    for selector in ["sequence", "source", "ledger"] {
        let mut input = corrected_posting(&f, &corrected, &new_ledger, "700");
        match selector {
            "sequence" => input.expected_sequence = old_k,
            "source" => input.distribution.expected_revision_id = Some(old.revision_id.clone()),
            "ledger" => {
                input.transaction.as_mut().unwrap().expected_revision_id =
                    Some(old_ledger.revision_id.clone())
            }
            _ => unreachable!(),
        }
        assert!(
            f.vault.post_company_distribution(input).is_err(),
            "{selector}"
        );
        assert_eq!(state(&f.vault), before, "Rollback {selector}");
    }
    let mut raw = journal(&f, "700");
    raw.id = Some(new_ledger.id.clone());
    raw.expected_revision_id = Some(new_ledger.revision_id.clone());
    assert!(f.vault.save_treasury_transaction(raw).is_err());
    assert_eq!(state(&f.vault), before);
}

#[test]
fn linked_dividend_cannot_create_or_move_to_another_existing_bank_operation() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let input = posting(&f, "400", None);
    let paid = f.vault.post_company_distribution(input).unwrap();
    let old_ledger = ledger(&f, &paid);
    let before = state(&f.vault);
    let mut duplicate = corrected_posting(&f, &paid, &old_ledger, "600");
    duplicate.transaction = Some(journal(&f, "600"));
    assert!(f.vault.post_company_distribution(duplicate).is_err());
    assert_eq!(state(&f.vault), before);
    let unrelated = journal(&f, "600");
    let unrelated = f.vault.save_treasury_transaction(unrelated).unwrap();
    let before = state(&f.vault);
    let mut moved = corrected_posting(&f, &paid, &old_ledger, "600");
    moved.transaction = None;
    moved.existing_transaction_revision_id = Some(unrelated.revision_id);
    assert!(f.vault.post_company_distribution(moved).is_err());
    assert_eq!(state(&f.vault), before);
    let mut detached = distribution(&f, "paid", "400", None);
    detached.id = Some(paid.id.clone());
    detached.expected_revision_id = Some(paid.revision_id.clone());
    detached.unlinked_reason =
        "Motif historique ne justifiant pas de détacher une source déjà liée".into();
    assert!(f.vault.save_company_distribution(detached).is_err());
    assert_eq!(state(&f.vault), before);
}

#[test]
fn declaration_limits_and_shared_lines_roll_back_sources_without_a_second_cash() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let declared = distribution(&f, "declared", "1000", None);
    let declared = f.vault.save_company_distribution(declared).unwrap();
    let raw = journal(&f, "1000");
    let raw = f.vault.save_treasury_transaction(raw).unwrap();
    let mut first = posting(&f, "600", Some(&declared));
    first.transaction = None;
    first.existing_transaction_revision_id = Some(raw.revision_id.clone());
    f.vault.post_company_distribution(first).unwrap();
    let before = state(&f.vault);
    let mut overcapacity = posting(&f, "500", None);
    overcapacity.transaction = None;
    overcapacity.existing_transaction_revision_id = Some(raw.revision_id.clone());
    assert!(f.vault.post_company_distribution(overcapacity).is_err());
    assert_eq!(state(&f.vault), before);
    let overdeclaration = posting(&f, "500", Some(&declared));
    assert!(f.vault.post_company_distribution(overdeclaration).is_err());
    assert_eq!(state(&f.vault), before);
    assert_eq!(balance(&f.vault, &f.payer, None), "9000.00");
    assert_eq!(balance(&f.vault, &f.recipient, None), "1000.00");
}

#[test]
fn company_and_metal_sources_share_the_same_exact_bank_line_capacity() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let raw = journal(&f, "1000");
    let raw = f.vault.save_treasury_transaction(raw).unwrap();
    let mut paid = posting(&f, "600", None);
    paid.transaction = None;
    paid.existing_transaction_revision_id = Some(raw.revision_id.clone());
    f.vault.post_company_distribution(paid).unwrap();
    let a = asset(&mut f.vault, &f.company, "Métal du contrôle de capacité");
    let lot = f
        .vault
        .save_metal_lot(SaveMetalLotInput {
            id: None,
            expected_revision_id: None,
            reason: "Lot synthétique".into(),
            asset_id: a.id,
            metal: "gold".into(),
            form: "bar".into(),
            designation: "Unité homogène".into(),
            unit_weight: "1".into(),
            weight_unit: "g".into(),
            purity: "1".into(),
            tracking_start: "2025-01-01".into(),
            cost_currency: "EUR".into(),
            legal_owner_entity_id: f.company.id.clone(),
            country: "FR".into(),
            storage_location: "Coffre synthétique".into(),
            valuation_basis: "intrinsic".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    let movement = SaveMetalMovementInput {
        id: None,
        expected_revision_id: None,
        reason: "Ouverture métal synthétique".into(),
        lot_id: lot.id,
        kind: "opening".into(),
        effective_on: "2025-01-01".into(),
        quantity_change: "0".into(),
        cost_amount: Some("0".into()),
        trade_amount: None,
        trade_fees: "0".into(),
        trade_currency: None,
        cash_entity_id: String::new(),
        counterparty_entity_id: None,
        country: "FR".into(),
        storage_location: "Coffre synthétique".into(),
        unlinked_reason: String::new(),
        notes: String::new(),
    };
    f.vault.save_metal_movement(movement.clone()).unwrap();
    let before = state(&f.vault);
    let movement = SaveMetalMovementInput {
        kind: "purchase".into(),
        effective_on: "2025-03-15".into(),
        quantity_change: "1".into(),
        cost_amount: Some("500".into()),
        trade_amount: Some("500".into()),
        trade_currency: Some("EUR".into()),
        cash_entity_id: f.company.id.clone(),
        ..movement
    };
    let input = PostMetalMovementInput {
        expected_sequence: f.vault.status().unwrap().mutation_sequence,
        movement,
        transaction: None,
        existing_transaction_revision_id: Some(raw.revision_id),
        links: vec![PostedMetalCashLink {
            line_id: "payer".into(),
            amount: "-500".into(),
        }],
    };
    let error = f.vault.post_metal_movement(input).unwrap_err();
    assert!(
        error.message.contains("capacité de la même ligne"),
        "Le rejet doit provenir de la capacité partagée, pas d’une source invalide : {error}"
    );
    assert_eq!(state(&f.vault), before);
}

#[test]
fn void_and_backup_preserve_distribution_child_links_and_prior_knowledge() {
    let tmp = TempDir::new().unwrap();
    let mut f = fixture(&tmp.path().join("v"));
    let declared = distribution(&f, "declared", "1000", None);
    let declared = f.vault.save_company_distribution(declared).unwrap();
    let input = posting(&f, "400", Some(&declared));
    let paid = f.vault.post_company_distribution(input).unwrap();
    let paid_k = f.vault.status().unwrap().mutation_sequence;
    f.vault
        .void_registry_record(VoidRecordInput {
            id: paid.id.clone(),
            expected_revision_id: paid.revision_id.clone(),
            reason: "Annuler la qualification, conserver le cash et les sources historiques".into(),
        })
        .unwrap();
    assert_eq!(
        f.vault.list_companies_at(None).unwrap().distributions.len(),
        1
    );
    assert_eq!(
        f.vault
            .list_companies_at(Some(paid_k))
            .unwrap()
            .distributions
            .iter()
            .find(|d| d.id == paid.id)
            .unwrap()
            .cash_links,
        paid.cash_links
    );
    assert_eq!(balance(&f.vault, &f.payer, None), "9600.00");
    let sql = connection(&f.vault);
    let void_revision: String = sql
        .query_row(
            "SELECT id FROM revisions WHERE object_id=?1 AND is_void=1",
            [&paid.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(sql.query_row("SELECT count(*) FROM company_distribution_cash_links WHERE distribution_revision_id=?1",[&void_revision],|r|r.get::<_,u64>(0)).unwrap(),2);
    let changed:u64=sql.query_row("SELECT count(*) FROM (SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=?1 EXCEPT SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=?2)",[&void_revision,&paid.revision_id],|r|r.get(0)).unwrap();
    assert_eq!(changed, 0);
    drop(sql);
    let before = state(&f.vault);
    let old = serde_json::to_value(f.vault.list_companies_at(Some(paid_k)).unwrap()).unwrap();
    let zip = tmp.path().join("company.zip");
    f.vault.create_backup(&zip).unwrap();
    let mut restored = Vault::open(tmp.path().join("restored")).unwrap();
    let inspected = restored.inspect_restore(&zip).unwrap();
    assert_eq!(inspected.schema_version, 6);
    restored
        .activate_restore(&inspected.token, true, None)
        .unwrap();
    assert_eq!(state(&restored), before);
    assert_eq!(
        serde_json::to_value(restored.list_companies_at(Some(paid_k)).unwrap()).unwrap(),
        old
    );
    assert_eq!(balance(&restored, &f.payer, None), "9600.00");
    assert_eq!(balance(&restored, &f.recipient, None), "400.00");
    let sql = connection(&restored);
    assert_eq!(
        sql.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert!(!sql
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
}
