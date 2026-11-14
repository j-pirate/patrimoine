//! Persistence oracles for the phase-5 rental source records, on synthetic vaults.
use patrimoine_application::{PropertyService, RegistryService, RentalService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn person(vault: &mut Vault, name: &str) -> Entity {
    vault
        .save_entity(SaveEntityInput {
            id: None,
            expected_revision_id: None,
            reason: "Fixture locative synthétique".into(),
            kind: "person".into(),
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

struct Fixture {
    vault: Vault,
    owner: Entity,
    tenant: Entity,
    asset: Asset,
    unit: RentalUnit,
    lease: Lease,
    terms: LeaseTerms,
}

fn fixture(root: &std::path::Path) -> Fixture {
    let mut vault = Vault::open(root).unwrap();
    let owner = person(&mut vault, "Bailleur synthétique");
    let tenant = person(&mut vault, "Locataire synthétique");
    vault
        .save_membership(SaveMembershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Famille synthétique".into(),
            person_id: owner.id.clone(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            relationship: String::new(),
        })
        .unwrap();
    let asset = vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Bien synthétique".into(),
            name: "Appartement locatif".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    vault
        .save_ownership(SaveOwnershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Droit synthétique".into(),
            asset_id: asset.id.clone(),
            owner_entity_id: owner.id.clone(),
            legal_share: Some(Fraction {
                num: "1".into(),
                den: "1".into(),
            }),
            economic_share: Some(Fraction {
                num: "1".into(),
                den: "1".into(),
            }),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    vault
        .save_property(SavePropertyInput {
            id: None,
            expected_revision_id: None,
            reason: "Fiche synthétique".into(),
            asset_id: asset.id.clone(),
            property_type: "apartment".into(),
            city: "Ville synthétique".into(),
            address: "Adresse synthétique".into(),
            surface_m2: Some("50".into()),
            status: "rented".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            notes: String::new(),
        })
        .unwrap();
    let unit = vault
        .save_rental_unit(SaveRentalUnitInput {
            id: None,
            expected_revision_id: None,
            reason: "Unité technique synthétique".into(),
            property_asset_id: asset.id.clone(),
            reference: "A".into(),
            name: "Logement entier".into(),
            surface_m2: Some("50".into()),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            available_for_rent: true,
            notes: String::new(),
        })
        .unwrap();
    let lease = vault
        .save_lease(SaveLeaseInput {
            id: None,
            expected_revision_id: None,
            reason: "Bail synthétique".into(),
            unit_id: unit.id.clone(),
            tenant_entity_id: tenant.id.clone(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            notes: String::new(),
        })
        .unwrap();
    let terms = vault
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
            valid_from: "2024-01-01".into(),
            valid_to: None,
            notes: String::new(),
        })
        .unwrap();
    Fixture {
        vault,
        owner,
        tenant,
        asset,
        unit,
        lease,
        terms,
    }
}

fn generate(f: &mut Fixture, from: &str, to: &str) -> RentGenerationResult {
    f.vault
        .generate_rent_dues(GenerateRentDuesInput {
            lease_id: f.lease.id.clone(),
            from_date: from.into(),
            to_date: to.into(),
            expected_sequence: f.vault.status().unwrap().mutation_sequence,
            reason: "Génération synthétique".into(),
        })
        .unwrap()
}

fn pay(f: &mut Fixture, amount: &str, date: &str) -> RentPayment {
    f.vault
        .save_rent_payment(SaveRentPaymentInput {
            id: None,
            expected_revision_id: None,
            reason: "Encaissement synthétique".into(),
            lease_id: f.lease.id.clone(),
            kind: "receipt".into(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: date.into(),
            original_payment_id: None,
            recipient_entity_id: f.owner.id.clone(),
            reference: "Reçu de test".into(),
            unlinked_reason: "Aucun compte bancaire n’est créé dans cette fixture de phase 5"
                .into(),
        })
        .unwrap()
}

fn allocation(
    payment: &RentPayment,
    due: &RentDue,
    amount: &str,
    date: &str,
) -> SaveRentAllocationInput {
    SaveRentAllocationInput {
        id: None,
        expected_revision_id: None,
        reason: "Affectation synthétique".into(),
        payment_id: payment.id.clone(),
        due_id: due.id.clone(),
        component: "rent".into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: date.into(),
    }
}

fn report(f: &Fixture, date: &str, k: Option<u64>) -> RentalReport {
    f.vault
        .get_rental_report(RentalReportRequest {
            context: "family".into(),
            person_id: None,
            asset_id: Some(f.asset.id.clone()),
            country: None,
            from_date: "2024-03-01".into(),
            to_date: "2024-04-01".into(),
            date: date.into(),
            knowledge_sequence: k,
        })
        .unwrap()
}

fn state(f: &Fixture) -> Value {
    serde_json::to_value(f.vault.list_rentals_at(None).unwrap()).unwrap()
}

#[test]
fn generation_is_atomic_one_change_and_idempotent_without_new_mutation() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let sequence = f.vault.status().unwrap().mutation_sequence;
    let first = generate(&mut f, "2024-01-01", "2024-04-01");
    assert_eq!(first.created.len(), 3);
    assert_eq!(first.existing_count, 0);
    assert_eq!(first.knowledge_sequence, sequence + 1);
    let before = state(&f);
    let second = generate(&mut f, "2024-01-01", "2024-04-01");
    assert!(second.created.is_empty());
    assert_eq!(second.existing_count, 3);
    assert_eq!(state(&f), before);
    assert_eq!(second.knowledge_sequence, first.knowledge_sequence);
}

#[test]
fn partial_payments_preserve_historical_outstanding_and_late_settlement() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let payment = pay(&mut f, "400", "2024-03-05");
    let a = allocation(&payment, &due, "400", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let old = f.vault.status().unwrap().mutation_sequence;
    let payment = pay(&mut f, "300", "2024-03-20");
    let a = allocation(&payment, &due, "300", "2024-03-20");
    f.vault.save_rent_allocation(a).unwrap();
    let r = report(&f, "2024-03-31", None);
    assert_eq!(r.rent_received_eur.as_deref(), Some("700.00"));
    assert_eq!(r.outstanding_rent_eur.as_deref(), Some("300.00"));
    let old_report = report(&f, "2024-03-31", Some(old));
    assert_eq!(old_report.rent_received_eur.as_deref(), Some("400.00"));
    assert_eq!(old_report.outstanding_rent_eur.as_deref(), Some("600.00"));
    let payment = pay(&mut f, "300", "2024-04-20");
    let a = allocation(&payment, &due, "300", "2024-04-20");
    f.vault.save_rent_allocation(a).unwrap();
    let march = report(&f, "2024-03-31", None);
    assert_eq!(march.outstanding_rent_eur.as_deref(), Some("300.00"));
    let april = report(&f, "2024-04-30", None);
    let row = april.dues.iter().find(|r| r.due_id == due.id).unwrap();
    assert_eq!(row.outstanding_rent, "0");
    assert!(row.historical_late);
    assert_eq!(row.settled_on.as_deref(), Some("2024-04-20"));
    assert!(april.cash_flow_after_financing_eur.is_none());
}

#[test]
fn one_receipt_covers_two_dues_without_duplicating_the_received_amount() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let dues = generate(&mut f, "2024-03-01", "2024-05-01").created;
    let march = dues
        .iter()
        .find(|d| d.period_start == "2024-03-01")
        .unwrap();
    let april = dues
        .iter()
        .find(|d| d.period_start == "2024-04-01")
        .unwrap();
    let payment = pay(&mut f, "1500", "2024-03-05");
    let a = allocation(&payment, march, "1000", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let a = allocation(&payment, april, "500", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let r = report(&f, "2024-03-31", None);
    assert_eq!(r.rent_received_eur.as_deref(), Some("1500.00"));
    let before = state(&f);
    let a = allocation(&payment, april, "1", "2024-03-05");
    assert!(f.vault.save_rent_allocation(a).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn overallocations_currency_and_future_payment_dates_reject_without_trace() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let payment = pay(&mut f, "1500", "2024-03-05");
    let before = state(&f);
    let a = allocation(&payment, &due, "1000.01", "2024-03-05");
    assert!(f.vault.save_rent_allocation(a).is_err());
    assert_eq!(state(&f), before);
    let mut a = allocation(&payment, &due, "100", "2024-03-05");
    a.currency = "TND".into();
    assert!(f.vault.save_rent_allocation(a).is_err());
    assert_eq!(state(&f), before);
    let a = allocation(&payment, &due, "100", "2024-03-04");
    assert!(f.vault.save_rent_allocation(a).is_err());
    assert_eq!(state(&f), before);
}

fn corrected_terms(f: &Fixture, rent: &str) -> SaveLeaseTermsInput {
    SaveLeaseTermsInput {
        id: Some(f.terms.id.clone()),
        expected_revision_id: Some(f.terms.revision_id.clone()),
        reason: "Correction motivée des conditions".into(),
        lease_id: f.lease.id.clone(),
        rent: rent.into(),
        charges: f.terms.charges.clone(),
        currency: f.terms.currency.clone(),
        frequency: f.terms.frequency.clone(),
        due_day: f.terms.due_day,
        deposit_required: f.terms.deposit_required.clone(),
        valid_from: f.terms.valid_from.clone(),
        valid_to: f.terms.valid_to.clone(),
        notes: String::new(),
    }
}

#[test]
fn corrected_terms_preserve_due_sources_and_generation_requires_explicit_regularisation() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let first = generate(&mut f, "2024-03-01", "2024-04-01");
    let old_sequence = first.knowledge_sequence;
    let input = corrected_terms(&f, "1100");
    let corrected = f.vault.save_lease_terms(input).unwrap();
    let snapshot = f.vault.list_rentals_at(None).unwrap();
    assert_eq!(snapshot.dues[0].terms_revision_id, f.terms.revision_id);
    assert!(snapshot
        .lease_term_revisions
        .iter()
        .any(|t| t.revision_id == corrected.revision_id));
    let historical = f.vault.list_rentals_at(Some(old_sequence)).unwrap();
    assert!(!historical
        .lease_term_revisions
        .iter()
        .any(|t| t.revision_id == corrected.revision_id));
    let before = state(&f);
    assert!(f
        .vault
        .generate_rent_dues(GenerateRentDuesInput {
            lease_id: f.lease.id.clone(),
            from_date: "2024-03-01".into(),
            to_date: "2024-04-01".into(),
            expected_sequence: snapshot.knowledge_sequence,
            reason: "Rejouer sans écraser les échéances".into()
        })
        .is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn stale_edits_and_stale_generation_are_atomic_conflicts() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let stale = f.vault.status().unwrap().mutation_sequence;
    let input = corrected_terms(&f, "1000,00");
    f.vault.save_lease_terms(input).unwrap();
    let before = state(&f);
    let input = corrected_terms(&f, "1200");
    assert!(f.vault.save_lease_terms(input).is_err());
    assert_eq!(state(&f), before);
    assert!(f
        .vault
        .generate_rent_dues(GenerateRentDuesInput {
            lease_id: f.lease.id.clone(),
            from_date: "2024-01-01".into(),
            to_date: "2024-02-01".into(),
            expected_sequence: stale,
            reason: "Génération périmée".into()
        })
        .is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn a_void_due_keeps_its_reserved_identity_and_key() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    f.vault
        .void_registry_record(VoidRecordInput {
            id: due.id,
            expected_revision_id: due.revision_id,
            reason: "Annulation synthétique à conserver".into(),
        })
        .unwrap();
    assert!(f.vault.list_rentals_at(None).unwrap().dues.is_empty());
    let before = state(&f);
    assert!(f
        .vault
        .generate_rent_dues(GenerateRentDuesInput {
            lease_id: f.lease.id.clone(),
            from_date: "2024-03-01".into(),
            to_date: "2024-04-01".into(),
            expected_sequence: f.vault.status().unwrap().mutation_sequence,
            reason: "Tentative de duplication de clé annulée".into()
        })
        .is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn leased_unit_and_lease_target_cannot_be_silently_changed_or_annulled() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let before = state(&f);
    assert!(f
        .vault
        .void_registry_record(VoidRecordInput {
            id: f.unit.id.clone(),
            expected_revision_id: f.unit.revision_id.clone(),
            reason: "Unité encore référencée".into()
        })
        .is_err());
    assert_eq!(state(&f), before);
    assert!(f
        .vault
        .save_lease(SaveLeaseInput {
            id: None,
            expected_revision_id: None,
            reason: "Chevauchement interdit".into(),
            unit_id: f.unit.id.clone(),
            tenant_entity_id: f.tenant.id.clone(),
            valid_from: "2024-02-01".into(),
            valid_to: None,
            notes: String::new()
        })
        .is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn archives_roundtrip_sources_keys_partial_payments_and_old_knowledge() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("vault");
    let mut f = fixture(&root);
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let payment = pay(&mut f, "400", "2024-03-05");
    let a = allocation(&payment, &due, "400", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let before = state(&f);
    let expected = report(&f, "2024-03-31", None);
    let old_k = f.vault.status().unwrap().mutation_sequence;
    let source = temp.path().join("original.txt");
    fs::write(&source, b"Original locatif synthetique").unwrap();
    f.vault
        .import_document(&source, "Bail synthétique")
        .unwrap();
    let archive = temp.path().join("backup.zip");
    f.vault.create_backup(&archive).unwrap();
    let at_backup = state(&f);
    let payment = pay(&mut f, "300", "2024-03-20");
    let a = allocation(&payment, &due, "300", "2024-03-20");
    f.vault.save_rent_allocation(a).unwrap();
    let inspection = f.vault.inspect_restore(&archive).unwrap();
    f.vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(state(&f), at_backup);
    assert_eq!(
        report(&f, "2024-03-31", None).outstanding_rent_eur,
        expected.outstanding_rent_eur
    );
    assert_eq!(
        serde_json::to_value(f.vault.list_rentals_at(Some(old_k)).unwrap()).unwrap(),
        before
    );
    assert_eq!(f.vault.list_documents().unwrap().len(), 1);
    drop(f);
    let reopened = Vault::open(&root).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.list_rentals_at(None).unwrap()).unwrap(),
        at_backup
    );
}

#[test]
fn future_contract_source_is_rejected_even_with_valid_sql_foreign_keys() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("vault");
    let mut f = fixture(&root);
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let input = corrected_terms(&f, "1100");
    let corrected = f.vault.save_lease_terms(input).unwrap();
    drop(f);
    let generation = fs::read_to_string(root.join("CURRENT")).unwrap();
    let c = Connection::open(
        root.join("generations")
            .join(generation.trim())
            .join("patrimoine.sqlite3"),
    )
    .unwrap();
    c.execute(
        "UPDATE rent_dues SET terms_revision_id=?1 WHERE revision_id=?2",
        [&corrected.revision_id, &due.revision_id],
    )
    .unwrap();
    drop(c);
    assert!(Vault::open(&root).is_err());
}

fn refund(
    f: &Fixture,
    original: &RentPayment,
    amount: &str,
    date: &str,
    allocations: Vec<RefundRentAllocationInput>,
) -> RefundRentPaymentInput {
    RefundRentPaymentInput {
        expected_sequence: f.vault.status().unwrap().mutation_sequence,
        payment: SaveRentPaymentInput {
            id: None,
            expected_revision_id: None,
            reason: "Remboursement synthétique motivé".into(),
            lease_id: f.lease.id.clone(),
            kind: "refund".into(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: date.into(),
            original_payment_id: Some(original.id.clone()),
            recipient_entity_id: f.owner.id.clone(),
            reference: "Remboursement de test".into(),
            unlinked_reason: "Phase 5 : aucune écriture bancaire dupliquée".into(),
        },
        allocations,
    }
}

#[test]
fn allocated_receipt_is_refunded_atomically_with_reopened_due_and_one_change_set() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let payment = pay(&mut f, "500", "2024-03-05");
    let a = allocation(&payment, &due, "500", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let before = state(&f);
    let old_k = f.vault.status().unwrap().mutation_sequence;
    let invalid = refund(&f, &payment, "-200", "2024-03-20", Vec::new());
    assert!(f.vault.save_rent_payment(invalid.payment).is_err());
    assert_eq!(state(&f), before);
    let input = refund(
        &f,
        &payment,
        "-200",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-200".into(),
        }],
    );
    let refunded = f.vault.refund_rent_payment(input).unwrap();
    assert_eq!(refunded.amount, "-200");
    assert_eq!(f.vault.status().unwrap().mutation_sequence, old_k + 1);
    let r = report(&f, "2024-03-31", None);
    assert_eq!(r.rent_received_eur.as_deref(), Some("300.00"));
    assert_eq!(r.outstanding_rent_eur.as_deref(), Some("700.00"));
    assert_eq!(
        report(&f, "2024-03-31", Some(old_k))
            .outstanding_rent_eur
            .as_deref(),
        Some("500.00")
    );
    let saved = state(&f);
    let backup = temp.path().join("refunded.zip");
    f.vault.create_backup(&backup).unwrap();
    let inspection = f.vault.inspect_restore(&backup).unwrap();
    f.vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(state(&f), saved);
    assert_eq!(
        report(&f, "2024-03-31", None)
            .outstanding_rent_eur
            .as_deref(),
        Some("700.00")
    );
}

#[test]
fn invalid_atomic_refund_leaves_neither_payment_allocation_nor_sequence() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let payment = pay(&mut f, "500", "2024-03-05");
    let a = allocation(&payment, &due, "500", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let before = state(&f);
    for amount in ["-250", "200"] {
        let input = refund(
            &f,
            &payment,
            "-200",
            "2024-03-20",
            vec![RefundRentAllocationInput {
                due_id: due.id.clone(),
                component: "rent".into(),
                amount: amount.into(),
            }],
        );
        assert!(f.vault.refund_rent_payment(input).is_err());
        assert_eq!(state(&f), before);
    }
    let input = refund(
        &f,
        &payment,
        "-200",
        "2024-03-04",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-200".into(),
        }],
    );
    assert!(f.vault.refund_rent_payment(input).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn refunded_unallocated_cash_cannot_be_reallocated_as_if_still_received() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let dues = generate(&mut f, "2024-03-01", "2024-05-01").created;
    let march = dues
        .iter()
        .find(|d| d.period_start == "2024-03-01")
        .unwrap();
    let april = dues
        .iter()
        .find(|d| d.period_start == "2024-04-01")
        .unwrap();
    let payment = pay(&mut f, "1500", "2024-03-05");
    let a = allocation(&payment, march, "1000", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let input = refund(&f, &payment, "-200", "2024-03-20", Vec::new());
    f.vault.refund_rent_payment(input).unwrap();
    let before = state(&f);
    let a = allocation(&payment, april, "400", "2024-03-21");
    assert!(f.vault.save_rent_allocation(a).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn refund_cannot_reverse_another_receipts_component() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let mut input = corrected_terms(&f, "1000");
    input.charges = "500".into();
    f.terms = f.vault.save_lease_terms(input).unwrap();
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let first = pay(&mut f, "500", "2024-03-05");
    let second = pay(&mut f, "900", "2024-03-05");
    let a = allocation(&first, &due, "100", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let mut a = allocation(&first, &due, "400", "2024-03-05");
    a.component = "charges".into();
    f.vault.save_rent_allocation(a).unwrap();
    let a = allocation(&second, &due, "900", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let before = state(&f);
    let input = refund(
        &f,
        &first,
        "-200",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-200".into(),
        }],
    );
    assert!(f.vault.refund_rent_payment(input).is_err());
    assert_eq!(state(&f), before);
}

#[test]
fn correcting_refund_replaces_its_allocations_atomically_and_preserves_old_knowledge() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let receipt = pay(&mut f, "500", "2024-03-05");
    let a = allocation(&receipt, &due, "500", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let input = refund(
        &f,
        &receipt,
        "-200",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-200".into(),
        }],
    );
    let original = f.vault.refund_rent_payment(input).unwrap();
    let old = f.vault.list_rentals_at(None).unwrap();
    let old_k = old.knowledge_sequence;
    let previous_allocation = old
        .allocations
        .iter()
        .find(|a| a.payment_id == original.id)
        .unwrap()
        .clone();
    let mut input = refund(
        &f,
        &receipt,
        "-100",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-100".into(),
        }],
    );
    input.payment.id = Some(original.id.clone());
    input.payment.expected_revision_id = Some(original.revision_id.clone());
    input.payment.reason = "Correction motivée du montant remboursé et de ses lignes".into();
    let corrected = f.vault.refund_rent_payment(input).unwrap();
    assert_eq!(corrected.id, original.id);
    assert_ne!(corrected.revision_id, original.revision_id);
    assert_eq!(corrected.amount, "-100");
    let current = f.vault.list_rentals_at(None).unwrap();
    assert_eq!(current.knowledge_sequence, old_k + 1);
    assert!(!current
        .allocations
        .iter()
        .any(|a| a.id == previous_allocation.id));
    let replacement: Vec<_> = current
        .allocations
        .iter()
        .filter(|a| a.payment_id == original.id)
        .collect();
    assert_eq!(replacement.len(), 1);
    assert_eq!(replacement[0].amount, "-100");
    assert_ne!(replacement[0].id, previous_allocation.id);
    assert_eq!(
        serde_json::to_value(f.vault.list_rentals_at(Some(old_k)).unwrap()).unwrap(),
        serde_json::to_value(old).unwrap()
    );
    assert_eq!(
        report(&f, "2024-03-31", Some(old_k))
            .outstanding_rent_eur
            .as_deref(),
        Some("700.00")
    );
    assert_eq!(
        report(&f, "2024-03-31", None)
            .outstanding_rent_eur
            .as_deref(),
        Some("600.00")
    );
    let backup = temp.path().join("corrected-refund.zip");
    f.vault.create_backup(&backup).unwrap();
    let saved = state(&f);
    let inspected = f.vault.inspect_restore(&backup).unwrap();
    f.vault
        .activate_restore(&inspected.token, true, None)
        .unwrap();
    assert_eq!(state(&f), saved);
}

#[test]
fn stale_global_sequence_or_payment_revision_cannot_replace_refund_allocations() {
    let temp = TempDir::new().unwrap();
    let mut f = fixture(&temp.path().join("vault"));
    let due = generate(&mut f, "2024-03-01", "2024-04-01")
        .created
        .remove(0);
    let receipt = pay(&mut f, "500", "2024-03-05");
    let a = allocation(&receipt, &due, "500", "2024-03-05");
    f.vault.save_rent_allocation(a).unwrap();
    let input = refund(
        &f,
        &receipt,
        "-200",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-200".into(),
        }],
    );
    let original = f.vault.refund_rent_payment(input).unwrap();
    let mut stale = refund(
        &f,
        &receipt,
        "-100",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id.clone(),
            component: "rent".into(),
            amount: "-100".into(),
        }],
    );
    stale.payment.id = Some(original.id.clone());
    stale.payment.expected_revision_id = Some(original.revision_id.clone());
    pay(&mut f, "10", "2024-03-21");
    let before = state(&f);
    assert!(f.vault.refund_rent_payment(stale).is_err());
    assert_eq!(state(&f), before);
    let mut invalid = refund(
        &f,
        &receipt,
        "-100",
        "2024-03-20",
        vec![RefundRentAllocationInput {
            due_id: due.id,
            component: "rent".into(),
            amount: "-100".into(),
        }],
    );
    invalid.payment.id = Some(original.id);
    invalid.payment.expected_revision_id = Some("00000000-0000-0000-0000-000000000000".into());
    assert!(f.vault.refund_rent_payment(invalid).is_err());
    assert_eq!(state(&f), before);
}
