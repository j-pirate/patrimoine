//! Independent persistence oracles for exact local cash and source links.
use patrimoine_application::{
    FinancialService, PropertyService, RegistryService, RentalService, TreasuryService,
    VaultService,
};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use serde_json::Value;
use tempfile::TempDir;
use uuid::Uuid;

fn id() -> String {
    Uuid::new_v4().to_string()
}
fn fraction() -> Option<Fraction> {
    Some(Fraction {
        num: "1".into(),
        den: "1".into(),
    })
}
fn person(v: &mut Vault) -> Entity {
    v.save_entity(SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Fixture synthétique".into(),
        kind: "person".into(),
        display_name: "Titulaire synthétique".into(),
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
            reason: "Fixture synthétique".into(),
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
        legal_share: fraction(),
        economic_share: fraction(),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    })
    .unwrap();
    a
}
fn account(
    v: &mut Vault,
    owner: &Entity,
    name: &str,
    currency: &str,
    kind: &str,
) -> TreasuryAccount {
    let a = asset(v, owner, name);
    v.save_treasury_account(SaveTreasuryAccountInput {
        id: None,
        expected_revision_id: None,
        reason: "Compte synthétique".into(),
        asset_id: a.id,
        kind: kind.into(),
        currency: currency.into(),
        tracking_start: "2025-01-01".into(),
        legal_owner_entity_id: owner.id.clone(),
        bank_name: if kind == "bank" {
            "Banque de test".into()
        } else {
            String::new()
        },
        reference: name.into(),
        country: "FR".into(),
        storage_location: if kind == "cash" {
            "Caisse de test".into()
        } else {
            String::new()
        },
        notes: String::new(),
        archived: false,
    })
    .unwrap()
}
fn entry(a: &TreasuryAccount, amount: &str, role: &str) -> TreasuryEntry {
    TreasuryEntry {
        line_id: id(),
        account_id: a.id.clone(),
        amount: amount.into(),
        role: role.into(),
    }
}
fn transaction(
    kind: &str,
    date: &str,
    entries: Vec<TreasuryEntry>,
) -> SaveTreasuryTransactionInput {
    SaveTreasuryTransactionInput {
        id: None,
        expected_revision_id: None,
        reason: "Écriture synthétique".into(),
        kind: kind.into(),
        effective_on: date.into(),
        label: "Journal de validation".into(),
        reference: String::new(),
        notes: String::new(),
        entries,
    }
}
fn opening(v: &mut Vault, a: &TreasuryAccount, amount: &str) {
    v.save_treasury_transaction(transaction(
        "opening",
        "2025-01-01",
        vec![entry(a, amount, "opening")],
    ))
    .unwrap();
}
fn report(v: &Vault, a: &TreasuryAccount, date: &str) -> TreasuryReport {
    v.get_treasury_report(TreasuryReportRequest {
        account_id: a.id.clone(),
        date: date.into(),
        knowledge_sequence: None,
    })
    .unwrap()
}
fn state(v: &Vault) -> Value {
    serde_json::json!({"treasury":v.list_treasury_at(None).unwrap(),"registry":v.list_registry().unwrap(),"rentals":v.list_rentals_at(None).unwrap(),"history":v.list_registry_history().unwrap()})
}

#[test]
fn opening_receipt_payment_and_internal_transfer_are_exact_and_reproducible_at_k() {
    let tmp = TempDir::new().unwrap();
    let mut v = Vault::open(tmp.path().join("v")).unwrap();
    let owner = person(&mut v);
    let a = account(&mut v, &owner, "Source", "EUR", "bank");
    let b = account(&mut v, &owner, "Destination", "EUR", "bank");
    opening(&mut v, &a, "100");
    opening(&mut v, &b, "0");
    v.save_treasury_transaction(transaction(
        "receipt",
        "2025-01-02",
        vec![entry(&a, "30", "principal")],
    ))
    .unwrap();
    v.save_treasury_transaction(transaction(
        "payment",
        "2025-01-03",
        vec![entry(&a, "-20", "principal")],
    ))
    .unwrap();
    let before = v.list_treasury_at(None).unwrap().knowledge_sequence;
    let transfer = v
        .save_treasury_transaction(transaction(
            "transfer",
            "2025-01-04",
            vec![entry(&a, "-50", "principal"), entry(&b, "50", "principal")],
        ))
        .unwrap();
    assert_eq!(
        report(&v, &a, "2025-01-04").balance_original.as_deref(),
        Some("60.00")
    );
    assert_eq!(
        report(&v, &b, "2025-01-04").balance_original.as_deref(),
        Some("50.00")
    );
    let old = v
        .get_treasury_report(TreasuryReportRequest {
            account_id: a.id.clone(),
            date: "2025-01-04".into(),
            knowledge_sequence: Some(before),
        })
        .unwrap();
    assert_eq!(old.balance_original.as_deref(), Some("110.00"));
    assert!(report(&v, &a, "2025-01-04")
        .source_ids
        .contains(&transfer.revision_id));
    let unchanged = state(&v);
    assert!(v
        .save_treasury_transaction(transaction(
            "transfer",
            "2025-01-05",
            vec![entry(&a, "-51", "principal"), entry(&b, "50", "principal")]
        ))
        .is_err());
    assert_eq!(state(&v), unchanged);
    assert!(v
        .save_treasury_transaction(transaction(
            "opening",
            "2025-01-01",
            vec![entry(&a, "100", "opening")]
        ))
        .is_err());
    assert_eq!(state(&v), unchanged);
}

#[test]
fn executed_fx_preserves_principals_exact_quotient_and_separate_fee_once() {
    let tmp = TempDir::new().unwrap();
    let mut v = Vault::open(tmp.path().join("v")).unwrap();
    let owner = person(&mut v);
    let eur = account(&mut v, &owner, "EUR", "EUR", "bank");
    let tnd = account(&mut v, &owner, "TND", "TND", "cash");
    opening(&mut v, &eur, "1000");
    opening(&mut v, &tnd, "0");
    v.save_treasury_transaction(transaction(
        "fx",
        "2025-01-05",
        vec![
            entry(&eur, "-100", "principal"),
            entry(&tnd, "330", "principal"),
            entry(&eur, "-2", "fee"),
        ],
    ))
    .unwrap();
    let result = report(&v, &eur, "2025-01-05");
    assert_eq!(result.balance_original.as_deref(), Some("898.00"));
    assert_eq!(
        result.executed_fx[0].exact_rate,
        Fraction {
            num: "33".into(),
            den: "10".into()
        }
    );
    assert_eq!(
        result.executed_fx[0].fees[0].amount,
        Fraction {
            num: "2".into(),
            den: "1".into()
        }
    );
    let foreign = report(&v, &tnd, "2025-01-05");
    assert_eq!(foreign.balance_original.as_deref(), Some("330.000"));
    assert_eq!(foreign.balance_eur, None);
    v.save_exchange_rate(SaveExchangeRateInput {
        id: None,
        expected_revision_id: None,
        reason: "Cours synthétique".into(),
        base_currency: "TND".into(),
        quote_currency: "EUR".into(),
        rate: "0.31".into(),
        effective_on: "2025-01-01".into(),
        source: Some("Source de test".into()),
        notes: String::new(),
    })
    .unwrap();
    assert_eq!(
        report(&v, &tnd, "2025-01-05").balance_eur.as_deref(),
        Some("102.30")
    );
    assert_eq!(
        report(&v, &eur, "2025-01-05").executed_fx[0].exact_rate,
        Fraction {
            num: "33".into(),
            den: "10".into()
        }
    );
}

#[test]
fn missing_opening_remains_unknown_reconciliation_is_only_an_observation_and_cash_cannot_be_negative(
) {
    let tmp = TempDir::new().unwrap();
    let mut v = Vault::open(tmp.path().join("v")).unwrap();
    let owner = person(&mut v);
    let bank = account(&mut v, &owner, "Banque", "EUR", "bank");
    assert_eq!(report(&v, &bank, "2025-01-01").balance_original, None);
    opening(&mut v, &bank, "110");
    v.save_account_reconciliation(SaveAccountReconciliationInput {
        id: None,
        expected_revision_id: None,
        reason: "Relevé synthétique".into(),
        account_id: bank.id.clone(),
        effective_on: "2025-01-31".into(),
        statement_balance: "100".into(),
        statement_reference: "RELEVE-TEST".into(),
        document_id: None,
        notes: String::new(),
    })
    .unwrap();
    let r = report(&v, &bank, "2025-01-31");
    assert_eq!(r.balance_original.as_deref(), Some("110.00"));
    assert_eq!(r.reconciliations[0].difference.as_deref(), Some("-10.00"));
    v.save_treasury_transaction(transaction(
        "payment",
        "2025-02-01",
        vec![entry(&bank, "-120", "principal")],
    ))
    .unwrap();
    assert!(report(&v, &bank, "2025-02-01").overdraft);
    let cash = account(&mut v, &owner, "Caisse", "EUR", "cash");
    opening(&mut v, &cash, "10");
    let unchanged = state(&v);
    assert!(v
        .save_treasury_transaction(transaction(
            "payment",
            "2025-02-01",
            vec![entry(&cash, "-11", "principal")]
        ))
        .is_err());
    assert_eq!(state(&v), unchanged);
}

struct RentalFixture {
    v: Vault,
    owner: Entity,
    bank: TreasuryAccount,
    lease: Lease,
    due: RentDue,
}
fn rental_fixture(path: &std::path::Path) -> RentalFixture {
    let mut v = Vault::open(path).unwrap();
    let owner = person(&mut v);
    let tenant = person(&mut v);
    let bank = account(&mut v, &owner, "Compte loyers", "EUR", "bank");
    opening(&mut v, &bank, "0");
    let a = asset(&mut v, &owner, "Bien locatif");
    v.save_property(SavePropertyInput {
        id: None,
        expected_revision_id: None,
        reason: "Fiche synthétique".into(),
        asset_id: a.id.clone(),
        property_type: "house".into(),
        city: String::new(),
        address: String::new(),
        surface_m2: None,
        status: "rented".into(),
        valid_from: "2020-01-01".into(),
        valid_to: None,
        notes: String::new(),
    })
    .unwrap();
    let unit = v
        .save_rental_unit(SaveRentalUnitInput {
            id: None,
            expected_revision_id: None,
            reason: "Unité synthétique".into(),
            property_asset_id: a.id,
            reference: "LOT".into(),
            name: "Unité de test".into(),
            surface_m2: None,
            valid_from: "2020-01-01".into(),
            valid_to: None,
            available_for_rent: true,
            notes: String::new(),
        })
        .unwrap();
    let lease = v
        .save_lease(SaveLeaseInput {
            id: None,
            expected_revision_id: None,
            reason: "Bail synthétique".into(),
            unit_id: unit.id,
            tenant_entity_id: tenant.id,
            valid_from: "2025-01-01".into(),
            valid_to: Some("2026-01-01".into()),
            notes: String::new(),
        })
        .unwrap();
    let terms = v
        .save_lease_terms(SaveLeaseTermsInput {
            id: None,
            expected_revision_id: None,
            reason: "Conditions synthétiques".into(),
            lease_id: lease.id.clone(),
            rent: "1000".into(),
            charges: "0".into(),
            currency: "EUR".into(),
            frequency: "monthly".into(),
            due_day: 5,
            deposit_required: "2000".into(),
            valid_from: "2025-01-01".into(),
            valid_to: Some("2026-01-01".into()),
            notes: String::new(),
        })
        .unwrap();
    let due = v
        .save_rent_due(SaveRentDueInput {
            id: None,
            expected_revision_id: None,
            reason: "Échéance synthétique".into(),
            lease_id: lease.id.clone(),
            terms_revision_id: terms.revision_id,
            period_start: "2025-03-01".into(),
            period_end: "2025-04-01".into(),
            due_on: "2025-03-05".into(),
            rent: "1000".into(),
            charges: "0".into(),
            currency: "EUR".into(),
            generation_key: "manual-test-march".into(),
            notes: String::new(),
        })
        .unwrap();
    RentalFixture {
        v,
        owner,
        bank,
        lease,
        due,
    }
}
fn payment_input(f: &RentalFixture, amount: &str) -> SaveRentPaymentInput {
    SaveRentPaymentInput {
        id: None,
        expected_revision_id: None,
        reason: "Réception synthétique".into(),
        lease_id: f.lease.id.clone(),
        kind: "receipt".into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: "2025-03-05".into(),
        original_payment_id: None,
        recipient_entity_id: f.owner.id.clone(),
        reference: "RENT-TEST".into(),
        unlinked_reason: String::new(),
    }
}
fn post_cash(a: &TreasuryAccount, date: &str, amount: &str, kind: &str) -> CashPostingInput {
    let line = entry(a, amount, "principal");
    CashPostingInput {
        line_id: line.line_id.clone(),
        transaction: Some(transaction(kind, date, vec![line])),
        existing_transaction_revision_id: None,
    }
}

#[test]
fn posting_rent_is_one_change_and_corrects_source_allocations_and_cash_atomically() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    let before = f.v.list_treasury_at(None).unwrap().knowledge_sequence;
    let p =
        f.v.post_rent_payment(PostRentPaymentInput {
            expected_sequence: before,
            payment: payment_input(&f, "400"),
            allocations: vec![PostedRentAllocation {
                due_id: f.due.id.clone(),
                component: "rent".into(),
                amount: "400".into(),
            }],
            cash: post_cash(&f.bank, "2025-03-05", "400", "rent"),
        })
        .unwrap();
    let after = f.v.list_treasury_at(None).unwrap();
    assert_eq!(after.knowledge_sequence, before + 1);
    assert_eq!(after.cash_links.len(), 1);
    assert_eq!(after.transactions.len(), 2);
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-31")
            .balance_original
            .as_deref(),
        Some("400.00")
    );
    let r = f.v.list_rentals_at(None).unwrap();
    assert_eq!(r.allocations[0].amount, "400");
    let old_cash = after
        .transactions
        .iter()
        .find(|t| t.kind == "rent")
        .unwrap()
        .clone();
    let unchanged = state(&f.v);
    let mut wrong = payment_input(&f, "450");
    wrong.id = Some(p.id.clone());
    wrong.expected_revision_id = Some(p.revision_id.clone());
    assert!(f.v.save_rent_payment(wrong.clone()).is_err());
    assert_eq!(state(&f.v), unchanged);
    let duplicate = f.v.post_rent_payment(PostRentPaymentInput {
        expected_sequence: after.knowledge_sequence,
        payment: wrong.clone(),
        allocations: Vec::new(),
        cash: post_cash(&f.bank, "2025-03-05", "450", "rent"),
    });
    assert!(duplicate.is_err());
    assert_eq!(state(&f.v), unchanged);
    let mut replacement = transaction(
        "rent",
        "2025-03-05",
        vec![TreasuryEntry {
            amount: "450".into(),
            ..old_cash.entries[0].clone()
        }],
    );
    replacement.id = Some(old_cash.id.clone());
    replacement.expected_revision_id = Some(old_cash.revision_id.clone());
    let corrected =
        f.v.post_rent_payment(PostRentPaymentInput {
            expected_sequence: after.knowledge_sequence,
            payment: wrong,
            allocations: vec![PostedRentAllocation {
                due_id: f.due.id.clone(),
                component: "rent".into(),
                amount: "450".into(),
            }],
            cash: CashPostingInput {
                transaction: Some(replacement),
                existing_transaction_revision_id: None,
                line_id: old_cash.entries[0].line_id.clone(),
            },
        })
        .unwrap();
    assert_eq!(corrected.id, p.id);
    assert_ne!(corrected.revision_id, p.revision_id);
    assert_eq!(f.v.list_treasury_at(None).unwrap().transactions.len(), 2);
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-31")
            .balance_original
            .as_deref(),
        Some("450.00")
    );
    assert_eq!(
        f.v.list_treasury_at(Some(after.knowledge_sequence))
            .unwrap()
            .cash_links[0]
            .source_revision_id,
        p.revision_id
    );
    let old =
        f.v.get_treasury_report(TreasuryReportRequest {
            account_id: f.bank.id.clone(),
            date: "2025-03-31".into(),
            knowledge_sequence: Some(after.knowledge_sequence),
        })
        .unwrap();
    assert_eq!(old.balance_original.as_deref(), Some("400.00"));
    let unchanged = state(&f.v);
    let mut stale = payment_input(&f, "450");
    stale.id = Some(p.id);
    stale.expected_revision_id = Some(p.revision_id);
    assert!(f
        .v
        .post_rent_payment(PostRentPaymentInput {
            expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
            payment: stale,
            allocations: Vec::new(),
            cash: CashPostingInput {
                transaction: None,
                existing_transaction_revision_id: Some(old_cash.revision_id),
                line_id: old_cash.entries[0].line_id.clone(),
            },
        })
        .is_err());
    assert_eq!(state(&f.v), unchanged);
}

#[test]
fn linked_rent_cannot_move_to_other_cash_or_revise_a_shared_transaction_alone() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    let ledger =
        f.v.save_treasury_transaction(transaction(
            "rent",
            "2025-03-05",
            vec![entry(&f.bank, "400", "principal")],
        ))
        .unwrap();
    let other =
        f.v.save_treasury_transaction(transaction(
            "rent",
            "2025-03-05",
            vec![entry(&f.bank, "200", "principal")],
        ))
        .unwrap();
    let mut payments = Vec::new();
    for _ in 0..2 {
        let payment =
            f.v.post_rent_payment(PostRentPaymentInput {
                expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
                payment: payment_input(&f, "200"),
                allocations: Vec::new(),
                cash: CashPostingInput {
                    transaction: None,
                    existing_transaction_revision_id: Some(ledger.revision_id.clone()),
                    line_id: ledger.entries[0].line_id.clone(),
                },
            })
            .unwrap();
        payments.push(payment);
    }
    let mut edit = payment_input(&f, "200");
    edit.id = Some(payments[0].id.clone());
    edit.expected_revision_id = Some(payments[0].revision_id.clone());
    let before = state(&f.v);
    assert!(f
        .v
        .post_rent_payment(PostRentPaymentInput {
            expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
            payment: edit.clone(),
            allocations: Vec::new(),
            cash: CashPostingInput {
                transaction: None,
                existing_transaction_revision_id: Some(other.revision_id),
                line_id: other.entries[0].line_id.clone(),
            },
        })
        .is_err());
    assert_eq!(state(&f.v), before);
    let mut replacement = transaction("rent", "2025-03-05", ledger.entries.clone());
    replacement.id = Some(ledger.id);
    replacement.expected_revision_id = Some(ledger.revision_id);
    assert!(f
        .v
        .post_rent_payment(PostRentPaymentInput {
            expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
            payment: edit,
            allocations: Vec::new(),
            cash: CashPostingInput {
                transaction: Some(replacement),
                existing_transaction_revision_id: None,
                line_id: ledger.entries[0].line_id.clone(),
            },
        })
        .is_err());
    assert_eq!(state(&f.v), before);
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-05")
            .balance_original
            .as_deref(),
        Some("600.00")
    );
}

#[test]
fn matching_existing_bank_line_does_not_create_cash_and_duplicate_source_or_line_rejects() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    let ledger =
        f.v.save_treasury_transaction(transaction(
            "rent",
            "2025-03-05",
            vec![entry(&f.bank, "400", "principal")],
        ))
        .unwrap();
    let mut p = payment_input(&f, "400");
    p.unlinked_reason = "Réception antérieure non rapprochée".into();
    let p = f.v.save_rent_payment(p).unwrap();
    let mut input = payment_input(&f, "400");
    input.id = Some(p.id.clone());
    input.expected_revision_id = Some(p.revision_id);
    let p =
        f.v.post_rent_payment(PostRentPaymentInput {
            expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
            payment: input,
            allocations: Vec::new(),
            cash: CashPostingInput {
                transaction: None,
                existing_transaction_revision_id: Some(ledger.revision_id.clone()),
                line_id: ledger.entries[0].line_id.clone(),
            },
        })
        .unwrap();
    assert_eq!(f.v.list_treasury_at(None).unwrap().transactions.len(), 2);
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-31")
            .balance_original
            .as_deref(),
        Some("400.00")
    );
    let unchanged = state(&f.v);
    assert!(f
        .v
        .save_cash_link(SaveCashLinkInput {
            id: None,
            expected_revision_id: None,
            reason: "Doublon invalide".into(),
            source_kind: "rent_payment".into(),
            source_id: p.id,
            source_revision_id: p.revision_id,
            transaction_revision_id: ledger.revision_id,
            line_id: ledger.entries[0].line_id.clone(),
            amount: "400".into(),
            component: "full".into(),
            side: String::new(),
            notes: String::new()
        })
        .is_err());
    assert_eq!(state(&f.v), unchanged);
}

#[test]
fn received_applied_and_returned_deposit_preserve_obligation_without_inventing_rent_cash() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    f.v.save_rent_claim(SaveRentClaimInput {
        id: None,
        expected_revision_id: None,
        reason: "Créancier de compensation explicitement identifié".into(),
        due_id: f.due.id.clone(),
        parties: vec![RentClaimParty {
            entity_id: f.owner.id.clone(),
            economic_share: fraction(),
        }],
        valid_from: "2025-03-05".into(),
        valid_to: None,
        represented_by_debt_id: None,
        notes: String::new(),
    })
    .unwrap();
    let deposit =
        |kind: &str, amount: &str, date: &str, due: Option<String>| SaveDepositMovementInput {
            id: None,
            expected_revision_id: None,
            reason: "Dépôt synthétique".into(),
            lease_id: f.lease.id.clone(),
            kind: kind.into(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: date.into(),
            recipient_entity_id: f.owner.id.clone(),
            component: if due.is_some() {
                "rent".into()
            } else {
                String::new()
            },
            due_id: due,
            notes: "Mouvement synthétique de dépôt motivé".into(),
        };
    f.v.post_deposit(PostDepositInput {
        expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
        movement: deposit("received", "2000", "2025-03-01", None),
        cash: Some(post_cash(&f.bank, "2025-03-01", "2000", "deposit")),
    })
    .unwrap();
    f.v.post_deposit(PostDepositInput {
        expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
        movement: deposit(
            "applied_to_due",
            "-300",
            "2025-03-10",
            Some(f.due.id.clone()),
        ),
        cash: None,
    })
    .unwrap();
    let t = f.v.list_treasury_at(None).unwrap();
    assert_eq!(t.transactions.len(), 2);
    assert_eq!(t.cash_links.len(), 1);
    assert!(f.v.list_rentals_at(None).unwrap().payments.is_empty());
    assert_eq!(
        deposit_obligation_at(&t, &f.lease.id, &f.owner.id, "2025-03-10")
            .unwrap()
            .to_string(),
        "1700"
    );
    assert_eq!(
        deposit_applied_at(&t, &f.due.id, "rent", "2025-03-10")
            .unwrap()
            .to_string(),
        "300"
    );
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-10")
            .balance_original
            .as_deref(),
        Some("2000.00")
    );
    let unchanged = state(&f.v);
    assert!(f
        .v
        .post_deposit(PostDepositInput {
            expected_sequence: t.knowledge_sequence,
            movement: deposit(
                "applied_to_due",
                "-701",
                "2025-03-11",
                Some(f.due.id.clone())
            ),
            cash: None
        })
        .is_err());
    assert_eq!(state(&f.v), unchanged);
    f.v.post_deposit(PostDepositInput {
        expected_sequence: t.knowledge_sequence,
        movement: deposit("returned", "-500", "2025-03-20", None),
        cash: Some(post_cash(&f.bank, "2025-03-20", "-500", "deposit")),
    })
    .unwrap();
    let t = f.v.list_treasury_at(None).unwrap();
    assert_eq!(
        deposit_obligation_at(&t, &f.lease.id, &f.owner.id, "2025-03-20")
            .unwrap()
            .to_string(),
        "1200"
    );
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-20")
            .balance_original
            .as_deref(),
        Some("1500.00")
    );
    let zip = tmp.path().join("backup.zip");
    f.v.create_backup(&zip).unwrap();
    let stored = state(&f.v);
    let token = f.v.inspect_restore(&zip).unwrap().token;
    f.v.activate_restore(&token, true, None).unwrap();
    assert_eq!(state(&f.v), stored);
}

#[test]
fn deposit_correction_replaces_its_cash_once_and_rejects_stale_or_isolated_changes() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    let mut movement = SaveDepositMovementInput {
        id: None,
        expected_revision_id: None,
        reason: "Réception documentée".into(),
        lease_id: f.lease.id.clone(),
        kind: "received".into(),
        amount: "100".into(),
        currency: "EUR".into(),
        effective_on: "2025-03-01".into(),
        recipient_entity_id: f.owner.id.clone(),
        due_id: None,
        component: String::new(),
        notes: "Reçu bancaire synthétique".into(),
    };
    let received =
        f.v.post_deposit(PostDepositInput {
            expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
            movement: movement.clone(),
            cash: Some(post_cash(&f.bank, "2025-03-01", "100", "deposit")),
        })
        .unwrap();
    let treasury = f.v.list_treasury_at(None).unwrap();
    let cash = treasury
        .transactions
        .iter()
        .find(|t| t.kind == "deposit")
        .unwrap()
        .clone();
    movement.id = Some(received.id);
    movement.expected_revision_id = Some(received.revision_id);
    movement.amount = "120".into();
    let before = state(&f.v);
    assert!(f.v.save_deposit_movement(movement.clone()).is_err());
    assert_eq!(state(&f.v), before);
    assert!(f
        .v
        .post_deposit(PostDepositInput {
            expected_sequence: treasury.knowledge_sequence,
            movement: movement.clone(),
            cash: Some(post_cash(&f.bank, "2025-03-01", "120", "deposit")),
        })
        .is_err());
    assert_eq!(state(&f.v), before);
    let mut replacement = transaction(
        "deposit",
        "2025-03-01",
        vec![TreasuryEntry {
            amount: "120".into(),
            ..cash.entries[0].clone()
        }],
    );
    replacement.id = Some(cash.id);
    replacement.expected_revision_id = Some(cash.revision_id.clone());
    let posted = CashPostingInput {
        transaction: Some(replacement),
        existing_transaction_revision_id: None,
        line_id: cash.entries[0].line_id.clone(),
    };
    assert!(f
        .v
        .post_deposit(PostDepositInput {
            expected_sequence: treasury.knowledge_sequence - 1,
            movement: movement.clone(),
            cash: Some(posted.clone()),
        })
        .is_err());
    assert_eq!(state(&f.v), before);
    let corrected =
        f.v.post_deposit(PostDepositInput {
            expected_sequence: treasury.knowledge_sequence,
            movement: movement.clone(),
            cash: Some(posted),
        })
        .unwrap();
    let current = f.v.list_treasury_at(None).unwrap();
    assert_eq!(current.knowledge_sequence, treasury.knowledge_sequence + 1);
    assert_eq!(current.transactions.len(), 2);
    assert_eq!(current.cash_links.len(), 1);
    assert_eq!(
        current.cash_links[0].source_revision_id,
        corrected.revision_id
    );
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-01")
            .balance_original
            .as_deref(),
        Some("120.00")
    );
    assert_eq!(
        deposit_obligation_at(&current, &f.lease.id, &f.owner.id, "2025-03-01")
            .unwrap()
            .to_string(),
        "120"
    );
    movement.expected_revision_id = Some(corrected.revision_id);
    let before = state(&f.v);
    assert!(f
        .v
        .post_deposit(PostDepositInput {
            expected_sequence: current.knowledge_sequence,
            movement,
            cash: Some(CashPostingInput {
                transaction: None,
                existing_transaction_revision_id: Some(cash.revision_id),
                line_id: cash.entries[0].line_id.clone(),
            }),
        })
        .is_err());
    assert_eq!(state(&f.v), before);
    assert_eq!(
        f.v.list_treasury_at(Some(treasury.knowledge_sequence))
            .unwrap()
            .deposits[0]
            .amount,
        "100"
    );
    let archive = tmp.path().join("corrected-deposit.zip");
    f.v.create_backup(&archive).unwrap();
    let mut restored = Vault::open(tmp.path().join("restored")).unwrap();
    let inspection = restored.inspect_restore(&archive).unwrap();
    assert_eq!(inspection.schema_version, 6);
    restored
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(state(&restored), state(&f.v));
    assert_eq!(
        serde_json::to_value(
            restored
                .list_treasury_at(Some(treasury.knowledge_sequence))
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(treasury).unwrap()
    );
}

#[test]
fn posting_paid_expense_keeps_negative_cash_and_source_corrections_atomic() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    let mut expense = SaveRentalExpenseInput {
        id: None,
        expected_revision_id: None,
        reason: "Facture réglée et justifiée".into(),
        asset_id: f.v.list_properties_at(None).unwrap().properties[0]
            .asset_id
            .clone(),
        kind: "operating".into(),
        amount: "80".into(),
        currency: "EUR".into(),
        effective_on: "2025-03-01".into(),
        paid_on: Some("2025-03-05".into()),
        recoverable: false,
        notes: "Justificatif synthétique".into(),
    };
    let before = f.v.list_treasury_at(None).unwrap().knowledge_sequence;
    let saved =
        f.v.post_rental_expense(PostRentalExpenseInput {
            expected_sequence: before,
            expense: expense.clone(),
            cash: post_cash(&f.bank, "2025-03-05", "-80", "payment"),
        })
        .unwrap();
    let treasury = f.v.list_treasury_at(None).unwrap();
    assert_eq!(treasury.knowledge_sequence, before + 1);
    assert_eq!(treasury.cash_links[0].amount, "-80");
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-05")
            .balance_original
            .as_deref(),
        Some("-80.00")
    );
    let cash = treasury
        .transactions
        .iter()
        .find(|t| t.kind == "payment")
        .unwrap()
        .clone();
    expense.id = Some(saved.id);
    expense.expected_revision_id = Some(saved.revision_id);
    expense.amount = "90".into();
    let unchanged = state(&f.v);
    assert!(f.v.save_rental_expense(expense.clone()).is_err());
    assert_eq!(state(&f.v), unchanged);
    assert!(f
        .v
        .post_rental_expense(PostRentalExpenseInput {
            expected_sequence: treasury.knowledge_sequence,
            expense: expense.clone(),
            cash: post_cash(&f.bank, "2025-03-05", "-90", "payment"),
        })
        .is_err());
    assert_eq!(state(&f.v), unchanged);
    let mut replacement = transaction(
        "payment",
        "2025-03-05",
        vec![TreasuryEntry {
            amount: "-90".into(),
            ..cash.entries[0].clone()
        }],
    );
    replacement.id = Some(cash.id);
    replacement.expected_revision_id = Some(cash.revision_id);
    let corrected =
        f.v.post_rental_expense(PostRentalExpenseInput {
            expected_sequence: treasury.knowledge_sequence,
            expense,
            cash: CashPostingInput {
                transaction: Some(replacement),
                existing_transaction_revision_id: None,
                line_id: cash.entries[0].line_id.clone(),
            },
        })
        .unwrap();
    let current = f.v.list_treasury_at(None).unwrap();
    assert_eq!(current.knowledge_sequence, treasury.knowledge_sequence + 1);
    assert_eq!(current.transactions.len(), 2);
    assert_eq!(
        current.cash_links[0].source_revision_id,
        corrected.revision_id
    );
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-05")
            .balance_original
            .as_deref(),
        Some("-90.00")
    );
    assert_eq!(
        f.v.list_rentals_at(Some(treasury.knowledge_sequence))
            .unwrap()
            .expenses[0]
            .amount,
        "80"
    );
}

fn describe_finance(f: &mut RentalFixture) {
    let asset = f.v.list_properties_at(None).unwrap().properties[0]
        .asset_id
        .clone();
    f.v.save_valuation(SaveValuationInput {
        id: None,
        expected_revision_id: None,
        reason: "Valeur synthétique".into(),
        target_kind: "asset".into(),
        target_id: asset,
        amount: "200000".into(),
        currency: "EUR".into(),
        effective_on: "2025-01-01".into(),
        source: "Estimation personnelle synthétique".into(),
    })
    .unwrap();
    f.v.save_rent_claim(SaveRentClaimInput {
        id: None,
        expected_revision_id: None,
        reason: "Créancier explicite synthétique".into(),
        due_id: f.due.id.clone(),
        parties: vec![RentClaimParty {
            entity_id: f.owner.id.clone(),
            economic_share: fraction(),
        }],
        valid_from: "2025-03-05".into(),
        valid_to: None,
        represented_by_debt_id: None,
        notes: String::new(),
    })
    .unwrap();
    f.v.save_finance_coverage(SaveFinanceCoverageInput {
        id: None,
        expected_revision_id: None,
        reason: "Périmètre de test déclaré complet".into(),
        valid_from: "2025-01-01".into(),
        valid_to: None,
        assets_complete: true,
        debts_complete: true,
        rentals_complete: true,
        notes: "Jeu synthétique fermé : un bien, une banque, un bail, aucun prêt.".into(),
    })
    .unwrap();
}
fn financial(f: &RentalFixture, date: &str) -> FinancialReport {
    f.v.get_financial_report(FinancialReportRequest {
        context: "person".into(),
        person_id: Some(f.owner.id.clone()),
        date: date.into(),
        knowledge_sequence: None,
    })
    .unwrap()
}

#[test]
fn partial_rent_in_bank_and_remaining_explicit_claim_are_one_asset_not_two_receipts() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    describe_finance(&mut f);
    let initial = financial(&f, "2025-03-31");
    assert_eq!(initial.known_net_eur, "201000.00");
    f.v.post_rent_payment(PostRentPaymentInput {
        expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
        payment: payment_input(&f, "400"),
        allocations: vec![PostedRentAllocation {
            due_id: f.due.id.clone(),
            component: "rent".into(),
            amount: "400".into(),
        }],
        cash: post_cash(&f.bank, "2025-03-05", "400", "rent"),
    })
    .unwrap();
    let after = financial(&f, "2025-03-31");
    assert_eq!(after.known_assets_eur, "201000.00");
    assert_eq!(after.known_net_eur, "201000.00");
    assert_eq!(
        report(&f.v, &f.bank, "2025-03-31")
            .balance_original
            .as_deref(),
        Some("400.00")
    );
    let before = state(&f.v);
    assert!(f
        .v
        .save_rent_claim_adjustment(SaveRentClaimAdjustmentInput {
            id: None,
            expected_revision_id: None,
            reason: "Dépréciation excessive".into(),
            due_id: f.due.id.clone(),
            creditor_entity_id: f.owner.id.clone(),
            kind: "impairment".into(),
            amount: "-601".into(),
            effective_on: "2025-03-06".into(),
            source: "Estimation synthétique".into(),
            notes: String::new()
        })
        .is_err());
    assert_eq!(state(&f.v), before);
    f.v.save_rent_claim_adjustment(SaveRentClaimAdjustmentInput {
        id: None,
        expected_revision_id: None,
        reason: "Dépréciation tracée".into(),
        due_id: f.due.id.clone(),
        creditor_entity_id: f.owner.id.clone(),
        kind: "impairment".into(),
        amount: "-100".into(),
        effective_on: "2025-03-06".into(),
        source: "Estimation synthétique".into(),
        notes: String::new(),
    })
    .unwrap();
    assert_eq!(financial(&f, "2025-03-31").known_net_eur, "200900.00");
}

#[test]
fn unallocated_receipt_is_an_advance_liability_until_explicit_allocation() {
    let tmp = TempDir::new().unwrap();
    let mut f = rental_fixture(&tmp.path().join("v"));
    describe_finance(&mut f);
    let mut p = payment_input(&f, "1500");
    p.effective_on = "2025-03-01".into();
    let p =
        f.v.post_rent_payment(PostRentPaymentInput {
            expected_sequence: f.v.list_treasury_at(None).unwrap().knowledge_sequence,
            payment: p,
            allocations: Vec::new(),
            cash: post_cash(&f.bank, "2025-03-01", "1500", "rent"),
        })
        .unwrap();
    let advance = financial(&f, "2025-03-01");
    assert_eq!(advance.known_assets_eur, "201500.00");
    assert_eq!(advance.known_liabilities_eur, "1500.00");
    assert_eq!(advance.known_net_eur, "200000.00");
    f.v.save_rent_allocation(SaveRentAllocationInput {
        id: None,
        expected_revision_id: None,
        reason: "Affectation explicite à exigibilité".into(),
        payment_id: p.id,
        due_id: f.due.id.clone(),
        component: "rent".into(),
        amount: "1000".into(),
        currency: "EUR".into(),
        effective_on: "2025-03-05".into(),
    })
    .unwrap();
    let allocated = financial(&f, "2025-03-05");
    assert_eq!(allocated.known_assets_eur, "201500.00");
    assert_eq!(allocated.known_liabilities_eur, "500.00");
    assert_eq!(allocated.known_net_eur, "201000.00");
    assert_eq!(f.v.list_treasury_at(None).unwrap().transactions.len(), 2);
}
