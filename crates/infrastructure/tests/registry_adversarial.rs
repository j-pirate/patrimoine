//! Independent integration oracles for the dated registry and its persistence.
//! All fixtures are synthetic and live in fresh temporary directories.
use patrimoine_application::{RegistryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

fn ratio(num: &str, den: &str) -> Fraction {
    Fraction {
        num: num.into(),
        den: den.into(),
    }
}

fn entity_input(name: &str, kind: &str) -> SaveEntityInput {
    SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Création synthétique".into(),
        kind: kind.into(),
        display_name: name.into(),
        first_name: (kind == "person").then(|| name.into()),
        last_name: None,
        birth_date: None,
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}

fn edit_entity(entity: &Entity) -> SaveEntityInput {
    SaveEntityInput {
        id: Some(entity.id.clone()),
        expected_revision_id: Some(entity.revision_id.clone()),
        reason: "Correction synthétique".into(),
        kind: entity.kind.clone(),
        display_name: entity.display_name.clone(),
        first_name: entity.first_name.clone(),
        last_name: entity.last_name.clone(),
        birth_date: entity.birth_date.clone(),
        residence_country: entity.residence_country.clone(),
        preferred_currency: entity.preferred_currency.clone(),
        activity: entity.activity.clone(),
        notes: entity.notes.clone(),
        archived: entity.archived,
    }
}

fn member(vault: &mut Vault, person: &Entity, from: &str, to: Option<&str>) -> Membership {
    vault
        .save_membership(SaveMembershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Périmètre synthétique".into(),
            person_id: person.id.clone(),
            valid_from: from.into(),
            valid_to: to.map(str::to_owned),
            relationship: "Famille".into(),
        })
        .unwrap()
}

fn asset(vault: &mut Vault, name: &str) -> Asset {
    vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Actif synthétique".into(),
            name: name.into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap()
}

fn ownership_input(
    asset: &Asset,
    owner: &Entity,
    economic: Option<Fraction>,
    from: &str,
    to: Option<&str>,
) -> SaveOwnershipInput {
    SaveOwnershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Droit synthétique".into(),
        asset_id: asset.id.clone(),
        owner_entity_id: owner.id.clone(),
        legal_share: economic.clone(),
        economic_share: economic,
        valid_from: from.into(),
        valid_to: to.map(str::to_owned),
    }
}

fn participation_input(
    owner: &Entity,
    company: &Entity,
    share: Fraction,
    from: &str,
    to: Option<&str>,
) -> SaveParticipationInput {
    SaveParticipationInput {
        id: None,
        expected_revision_id: None,
        reason: "Participation synthétique".into(),
        owner_entity_id: owner.id.clone(),
        company_id: company.id.clone(),
        legal_share: Some(share.clone()),
        economic_share: Some(share.clone()),
        voting_share: Some(share),
        valid_from: from.into(),
        valid_to: to.map(str::to_owned),
    }
}

fn valuation(
    vault: &mut Vault,
    target: &str,
    amount: &str,
    currency: &str,
    date: &str,
) -> Valuation {
    vault
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Estimation synthétique".into(),
            target_kind: "asset".into(),
            target_id: target.into(),
            amount: amount.into(),
            currency: currency.into(),
            effective_on: date.into(),
            source: "Fixture indépendante".into(),
        })
        .unwrap()
}

fn request(
    person: Option<&Entity>,
    date: &str,
    knowledge_sequence: Option<u64>,
) -> ConsolidationRequest {
    ConsolidationRequest {
        context: if person.is_some() { "person" } else { "family" }.into(),
        person_id: person.map(|p| p.id.clone()),
        date: date.into(),
        knowledge_sequence,
    }
}

fn counters(vault: &Vault) -> Vec<i64> {
    let status = vault.status().unwrap();
    let path = Path::new(&status.data_dir)
        .join("generations")
        .join(status.generation_id)
        .join("patrimoine.sqlite3");
    let connection = Connection::open(path).unwrap();
    ["objects", "revisions", "change_sets", "audit_events"]
        .into_iter()
        .map(|table| {
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
}

fn registry_json(vault: &Vault) -> serde_json::Value {
    serde_json::to_value(vault.list_registry().unwrap()).unwrap()
}

#[test]
fn profile_correction_preserves_identity_and_reopens_with_its_new_revision() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("coffre");
    let mut vault = Vault::open(&root).unwrap();
    let first = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let mut input = edit_entity(&first);
    input.display_name = "Alice corrigée".into();
    let second = vault.save_entity(input).unwrap();
    assert_eq!(first.id, second.id);
    assert_ne!(first.revision_id, second.revision_id);
    drop(vault);
    let reopened = Vault::open(&root).unwrap();
    let rows = reopened.list_registry().unwrap().entities;
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].revision_id, second.revision_id);
    assert_eq!(rows[0].display_name, "Alice corrigée");
}

#[test]
fn stale_expected_revision_rolls_back_audit_objects_and_mutation_sequence() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let first = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let mut updated = edit_entity(&first);
    updated.notes = "Version actuelle".into();
    vault.save_entity(updated).unwrap();
    let before = registry_json(&vault);
    let counts = counters(&vault);
    let error = vault.save_entity(edit_entity(&first)).unwrap_err();
    assert_eq!(error.code, "conflict");
    assert_eq!(registry_json(&vault), before);
    assert_eq!(counters(&vault), counts);
}

#[test]
fn an_existing_person_cannot_be_retyped_as_a_company() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let before = registry_json(&vault);
    let counts = counters(&vault);
    let mut input = edit_entity(&person);
    input.kind = "company".into();
    assert!(vault.save_entity(input).is_err());
    assert_eq!(registry_json(&vault), before);
    assert_eq!(counters(&vault), counts);
}

#[test]
fn a_cycle_existing_only_in_the_past_is_rejected_atomically() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let x = vault.save_entity(entity_input("X", "company")).unwrap();
    let y = vault.save_entity(entity_input("Y", "company")).unwrap();
    vault
        .save_participation(participation_input(
            &x,
            &y,
            ratio("1", "2"),
            "2020-01-01",
            Some("2021-01-01"),
        ))
        .unwrap();
    let before = registry_json(&vault);
    let counts = counters(&vault);
    assert!(vault
        .save_participation(participation_input(
            &y,
            &x,
            ratio("1", "2"),
            "2020-06-01",
            Some("2020-06-02")
        ))
        .is_err());
    assert_eq!(registry_json(&vault), before);
    assert_eq!(counters(&vault), counts);
}

#[test]
fn a_cycle_existing_only_in_a_future_interval_is_rejected() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let x = vault.save_entity(entity_input("X", "company")).unwrap();
    let y = vault.save_entity(entity_input("Y", "company")).unwrap();
    vault
        .save_participation(participation_input(
            &x,
            &y,
            ratio("1", "2"),
            "2040-01-01",
            Some("2041-01-01"),
        ))
        .unwrap();
    let before = registry_json(&vault);
    assert!(vault
        .save_participation(participation_input(
            &y,
            &x,
            ratio("1", "2"),
            "2040-12-31",
            None
        ))
        .is_err());
    assert_eq!(registry_json(&vault), before);
}

#[test]
fn reciprocal_participations_in_disjoint_half_open_intervals_are_accepted() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let x = vault.save_entity(entity_input("X", "company")).unwrap();
    let y = vault.save_entity(entity_input("Y", "company")).unwrap();
    vault
        .save_participation(participation_input(
            &x,
            &y,
            ratio("1", "2"),
            "2020-01-01",
            Some("2021-01-01"),
        ))
        .unwrap();
    vault
        .save_participation(participation_input(
            &y,
            &x,
            ratio("1", "2"),
            "2021-01-01",
            None,
        ))
        .unwrap();
    assert_eq!(vault.list_registry().unwrap().participations.len(), 2);
}

#[test]
fn overlapping_memberships_are_refused_but_adjacent_intervals_are_valid() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    member(&mut vault, &person, "2020-01-01", Some("2021-01-01"));
    let before = registry_json(&vault);
    assert!(vault
        .save_membership(SaveMembershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Chevauchement à refuser".into(),
            person_id: person.id.clone(),
            valid_from: "2020-12-31".into(),
            valid_to: Some("2021-02-01".into()),
            relationship: String::new()
        })
        .is_err());
    assert_eq!(registry_json(&vault), before);
    member(&mut vault, &person, "2021-01-01", None);
    assert_eq!(vault.list_registry().unwrap().memberships.len(), 2);
}

#[test]
fn rights_exceeding_one_only_at_an_interior_date_are_refused() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let a = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let b = vault.save_entity(entity_input("Bob", "person")).unwrap();
    let property = asset(&mut vault, "Appartement");
    vault
        .save_ownership(ownership_input(
            &property,
            &a,
            Some(ratio("3", "5")),
            "2020-01-01",
            Some("2022-01-01"),
        ))
        .unwrap();
    let before = registry_json(&vault);
    let counts = counters(&vault);
    assert!(vault
        .save_ownership(ownership_input(
            &property,
            &b,
            Some(ratio("1", "2")),
            "2021-01-01",
            Some("2021-02-01")
        ))
        .is_err());
    assert_eq!(registry_json(&vault), before);
    assert_eq!(counters(&vault), counts);
    // Les contrôles du titre et du droit économique sont indépendants.
    let mut only_legal = ownership_input(
        &property,
        &b,
        Some(ratio("0", "1")),
        "2021-01-01",
        Some("2021-02-01"),
    );
    only_legal.legal_share = Some(ratio("1", "2"));
    assert!(vault.save_ownership(only_legal).is_err());
    let mut only_economic = ownership_input(
        &property,
        &b,
        Some(ratio("1", "2")),
        "2021-01-01",
        Some("2021-02-01"),
    );
    only_economic.legal_share = Some(ratio("0", "1"));
    assert!(vault.save_ownership(only_economic).is_err());
    assert_eq!(registry_json(&vault), before);
    assert_eq!(counters(&vault), counts);
}

#[test]
fn duplicate_overlapping_rights_of_the_same_holder_are_refused() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let property = asset(&mut vault, "Appartement");
    vault
        .save_ownership(ownership_input(
            &property,
            &person,
            Some(ratio("1", "4")),
            "2020-01-01",
            None,
        ))
        .unwrap();
    let before = registry_json(&vault);
    assert!(vault
        .save_ownership(ownership_input(
            &property,
            &person,
            Some(ratio("1", "4")),
            "2021-01-01",
            None
        ))
        .is_err());
    assert_eq!(registry_json(&vault), before);
}

#[test]
fn an_unknown_economic_right_is_preserved_and_not_inferred_from_its_legal_title() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    member(&mut vault, &person, "2020-01-01", None);
    let property = asset(&mut vault, "Appartement");
    valuation(&mut vault, &property.id, "400000", "EUR", "2020-01-01");
    let mut input = ownership_input(&property, &person, None, "2020-01-01", None);
    input.legal_share = Some(ratio("1", "1"));
    let saved = vault.save_ownership(input).unwrap();
    let third = vault
        .save_entity(entity_input("Tiers identifié", "person"))
        .unwrap();
    let mut outsider =
        ownership_input(&property, &third, Some(ratio("1", "1")), "2020-01-01", None);
    outsider.legal_share = None;
    vault.save_ownership(outsider).unwrap();
    assert!(saved.economic_share.is_none());
    let report = vault
        .get_consolidation(request(Some(&person), "2020-06-01", None))
        .unwrap();
    assert_ne!(report.status, "complete");
    assert_ne!(report.status, "complet");
    assert!(!report.warnings.is_empty());
    assert_ne!(
        report.gross_eur, "400000.00",
        "Le titre juridique ne doit pas remplacer le droit économique inconnu"
    );
    assert!(vault
        .list_registry()
        .unwrap()
        .ownerships
        .iter()
        .find(|o| o.id == saved.id)
        .unwrap()
        .economic_share
        .is_none());
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "unknown_economic_rights"));
    vault
        .save_ownership(SaveOwnershipInput {
            id: Some(saved.id.clone()),
            expected_revision_id: Some(saved.revision_id),
            reason: "Droit nul confirmé".into(),
            asset_id: saved.asset_id,
            owner_entity_id: saved.owner_entity_id,
            legal_share: saved.legal_share,
            economic_share: Some(ratio("0", "1")),
            valid_from: saved.valid_from,
            valid_to: saved.valid_to,
        })
        .unwrap();
    let known_zero = vault
        .get_consolidation(request(Some(&person), "2020-06-01", None))
        .unwrap();
    assert_eq!(known_zero.gross_eur, "0.00");
    assert!(!known_zero
        .warnings
        .iter()
        .any(|w| w.code == "unknown_economic_rights"));
}

#[test]
fn a_foreign_valuation_keeps_its_source_without_inventing_a_euro_rate() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    member(&mut vault, &person, "2020-01-01", None);
    let property = asset(&mut vault, "Appartement");
    vault
        .save_ownership(ownership_input(
            &property,
            &person,
            Some(ratio("1", "1")),
            "2020-01-01",
            None,
        ))
        .unwrap();
    let source = valuation(&mut vault, &property.id, "123456.789", "TND", "2020-01-01");
    let report = vault
        .get_consolidation(request(None, "2020-06-01", None))
        .unwrap();
    assert_eq!(report.gross_eur, "0.00");
    assert!(!report.warnings.is_empty());
    assert_ne!(report.status, "complete");
    let saved = vault.list_registry().unwrap().valuations;
    assert_eq!(saved[0].id, source.id);
    assert_eq!(saved[0].currency, "TND");
    assert_eq!(saved[0].amount, "123456.789");
}

#[test]
fn archiving_a_profile_does_not_erase_its_historical_or_current_economic_rights() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    member(&mut vault, &person, "2020-01-01", None);
    let property = asset(&mut vault, "Appartement");
    valuation(&mut vault, &property.id, "400000", "EUR", "2020-01-01");
    vault
        .save_ownership(ownership_input(
            &property,
            &person,
            Some(ratio("1", "1")),
            "2020-01-01",
            None,
        ))
        .unwrap();
    let before = vault
        .get_consolidation(request(None, "2020-06-01", None))
        .unwrap();
    let mut archived = edit_entity(&person);
    archived.archived = true;
    vault.save_entity(archived).unwrap();
    let after = vault
        .get_consolidation(request(None, "2020-06-01", None))
        .unwrap();
    assert_eq!(before.gross_eur, "400000.00");
    assert_eq!(before.exact_gross, after.exact_gross);
    assert!(vault.list_registry().unwrap().entities[0].archived);
}

#[test]
fn corrected_valuations_remain_reproducible_at_their_knowledge_sequence() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    member(&mut vault, &person, "2020-01-01", Some("2021-01-01"));
    let property = asset(&mut vault, "Appartement");
    vault
        .save_ownership(ownership_input(
            &property,
            &person,
            Some(ratio("1", "1")),
            "2020-01-01",
            None,
        ))
        .unwrap();
    let old = valuation(&mut vault, &property.id, "400000", "EUR", "2020-01-01");
    let knowledge = vault.status().unwrap().mutation_sequence;
    vault
        .save_valuation(SaveValuationInput {
            id: Some(old.id.clone()),
            expected_revision_id: Some(old.revision_id),
            reason: "Correction rétroactive de valeur".into(),
            target_kind: old.target_kind,
            target_id: old.target_id,
            amount: "450000".into(),
            currency: old.currency,
            effective_on: old.effective_on,
            source: "Correction justifiée".into(),
        })
        .unwrap();
    assert_eq!(
        vault
            .get_consolidation(request(None, "2020-06-01", Some(knowledge)))
            .unwrap()
            .gross_eur,
        "400000.00"
    );
    assert_eq!(
        vault
            .get_consolidation(request(None, "2020-06-01", None))
            .unwrap()
            .gross_eur,
        "450000.00"
    );
    assert_eq!(
        vault
            .get_consolidation(request(None, "2020-12-31", None))
            .unwrap()
            .gross_eur,
        "450000.00"
    );
    assert_eq!(
        vault
            .get_consolidation(request(None, "2021-01-01", None))
            .unwrap()
            .gross_eur,
        "0.00"
    );
}

#[test]
fn voiding_an_entity_cannot_leave_active_rights_pointing_to_a_missing_profile() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let person = vault.save_entity(entity_input("Alice", "person")).unwrap();
    member(&mut vault, &person, "2020-01-01", None);
    let before = registry_json(&vault);
    let counts = counters(&vault);
    assert!(vault
        .void_registry_record(VoidRecordInput {
            id: person.id,
            expected_revision_id: person.revision_id,
            reason: "Annulation incohérente à refuser".into()
        })
        .is_err());
    assert_eq!(registry_json(&vault), before);
    assert_eq!(counters(&vault), counts);
}

#[test]
fn a_registry_backup_restores_ids_revisions_fractions_history_and_calculated_results() {
    let temp = TempDir::new().unwrap();
    let mut source = Vault::open(temp.path().join("source")).unwrap();
    let a = source.save_entity(entity_input("Alice", "person")).unwrap();
    let b = source.save_entity(entity_input("Bob", "person")).unwrap();
    let third = source.save_entity(entity_input("Tiers", "person")).unwrap();
    member(&mut source, &a, "2020-01-01", None);
    member(&mut source, &b, "2020-01-01", None);
    let property = asset(&mut source, "Appartement commun");
    for (holder, share) in [
        (&a, ratio("1", "2")),
        (&b, ratio("3", "10")),
        (&third, ratio("1", "5")),
    ] {
        source
            .save_ownership(ownership_input(
                &property,
                holder,
                Some(share),
                "2020-01-01",
                None,
            ))
            .unwrap();
    }
    valuation(&mut source, &property.id, "400000", "EUR", "2020-01-01");
    let document_path = temp.path().join("acte.txt");
    fs::write(&document_path, "Acte synthétique à récupérer").unwrap();
    let document = source
        .import_document(&document_path, "Pièce de test")
        .unwrap();
    let original = registry_json(&source);
    let report = source
        .get_consolidation(request(None, "2020-06-01", None))
        .unwrap();
    assert_eq!(report.gross_eur, "320000.00");
    let backup = temp.path().join("registre.zip");
    source.create_backup(&backup).unwrap();
    let mut restored = Vault::open(temp.path().join("restaure")).unwrap();
    let inspection = restored.inspect_restore(&backup).unwrap();
    restored
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(registry_json(&restored), original);
    assert_eq!(
        restored
            .get_consolidation(request(None, "2020-06-01", None))
            .unwrap()
            .exact_gross,
        report.exact_gross
    );
    assert_eq!(
        restored
            .get_consolidation(request(Some(&a), "2020-06-01", None))
            .unwrap()
            .gross_eur,
        "200000.00"
    );
    assert_eq!(
        restored
            .get_consolidation(request(Some(&b), "2020-06-01", None))
            .unwrap()
            .gross_eur,
        "120000.00"
    );
    assert_eq!(restored.list_documents().unwrap()[0], document);
    let exported = temp.path().join("acte-recupere.txt");
    restored.export_document(&document.id, &exported).unwrap();
    assert_eq!(
        fs::read(exported).unwrap(),
        fs::read(document_path).unwrap()
    );
}

#[test]
fn persisted_multiple_paths_and_global_equity_boundary_never_add_the_same_underlying_twice() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let alice = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let outsider = vault.save_entity(entity_input("Tiers", "person")).unwrap();
    let x = vault.save_entity(entity_input("X", "company")).unwrap();
    let y = vault.save_entity(entity_input("Y", "company")).unwrap();
    member(&mut vault, &alice, "2020-01-01", None);
    for (holder, company, share) in [
        (&alice, &x, ratio("3", "5")),
        (&outsider, &x, ratio("2", "5")),
        (&x, &y, ratio("1", "2")),
        (&alice, &y, ratio("1", "5")),
        (&outsider, &y, ratio("3", "10")),
    ] {
        vault
            .save_participation(participation_input(
                holder,
                company,
                share,
                "2020-01-01",
                None,
            ))
            .unwrap();
    }
    let mut policies = Vec::new();
    for company in [&x, &y] {
        policies.push(
            vault
                .save_company_policy(SaveCompanyPolicyInput {
                    id: None,
                    expected_revision_id: None,
                    reason: "Couverture détaillée synthétique".into(),
                    company_id: company.id.clone(),
                    mode: "detailed".into(),
                    coverage_complete: true,
                    liability_qualified: false,
                    valid_from: "2020-01-01".into(),
                    valid_to: None,
                })
                .unwrap(),
        );
    }
    let flat = asset(&mut vault, "Appartement détenu par Y");
    vault
        .save_ownership(ownership_input(
            &flat,
            &y,
            Some(ratio("1", "1")),
            "2020-01-01",
            None,
        ))
        .unwrap();
    valuation(&mut vault, &flat.id, "400000", "EUR", "2020-01-01");
    let detailed = vault
        .get_consolidation(request(None, "2020-06-01", None))
        .unwrap();
    assert_eq!(
        detailed.gross_eur, "200000.00",
        "(60 % × 50 % + 20 %) × 400 000"
    );
    assert_eq!(
        detailed
            .contributions
            .iter()
            .filter(|c| c.object_id == flat.id)
            .count(),
        1
    );
    let old_policy = policies.remove(0);
    vault
        .save_company_policy(SaveCompanyPolicyInput {
            id: Some(old_policy.id),
            expected_revision_id: Some(old_policy.revision_id),
            reason: "Valeur globale des capitaux propres".into(),
            company_id: x.id.clone(),
            mode: "global_equity".into(),
            coverage_complete: true,
            liability_qualified: false,
            valid_from: "2020-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    vault
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Estimation synthétique de X".into(),
            target_kind: "company".into(),
            target_id: x.id.clone(),
            amount: "500000".into(),
            currency: "EUR".into(),
            effective_on: "2020-01-01".into(),
            source: "Capitaux propres incluant la participation dans Y".into(),
        })
        .unwrap();
    let opaque = vault
        .get_consolidation(request(None, "2020-06-01", None))
        .unwrap();
    assert_eq!(
        opaque.gross_eur, "380000.00",
        "60 % × 500 000 + 20 % × 400 000 ; le chemin X→Y s'arrête"
    );
    assert_eq!(opaque.global_equity_eur, "300000.00");
    assert!(!opaque.gross_is_underlying_complete);
    let flat_line = opaque
        .contributions
        .iter()
        .find(|c| c.object_id == flat.id)
        .unwrap();
    assert_eq!(flat_line.weight, ratio("1", "5"));
    assert_eq!(flat_line.amount_eur, "80000.00");
}

#[test]
fn recomputing_archive_hashes_does_not_authorize_a_profile_with_another_stable_identity() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let alice = vault.save_entity(entity_input("Alice", "person")).unwrap();
    let bob = vault.save_entity(entity_input("Bob", "person")).unwrap();
    let backup = temp.path().join("valid.zip");
    vault.create_backup(&backup).unwrap();
    let mut zip = ZipArchive::new(fs::File::open(&backup).unwrap()).unwrap();
    let mut entries = Vec::new();
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).unwrap();
        let name = file.name().unwrap().into_owned();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        entries.push((name, bytes));
    }
    let db_index = entries
        .iter()
        .position(|(name, _)| name == "data/patrimoine.sqlite3")
        .unwrap();
    let modified_path = temp.path().join("modified.sqlite3");
    fs::write(&modified_path, &entries[db_index].1).unwrap();
    let connection = Connection::open(&modified_path).unwrap();
    connection.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    assert_eq!(
        connection
            .execute(
                "UPDATE entity_profiles SET entity_id=?1 WHERE entity_id=?2",
                [&bob.id, &alice.id]
            )
            .unwrap(),
        1
    );
    // Every SQL foreign key remains valid: the profile now points to a different
    // existing person. The registry's identity constraint must still reject it.
    assert!(!connection
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    connection.close().unwrap();
    entries[db_index].1 = fs::read(modified_path).unwrap();
    let new_hash = format!("{:x}", Sha256::digest(&entries[db_index].1));
    let new_size = entries[db_index].1.len() as u64;
    let manifest_index = entries
        .iter()
        .position(|(name, _)| name == "manifest.json")
        .unwrap();
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&entries[manifest_index].1).unwrap();
    let db_manifest = manifest["files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["path"] == "data/patrimoine.sqlite3")
        .unwrap();
    db_manifest["sha256"] = new_hash.into();
    db_manifest["size_bytes"] = new_size.into();
    entries[manifest_index].1 = serde_json::to_vec(&manifest).unwrap();
    let hostile = temp.path().join("reidentified-profile.zip");
    let mut writer = ZipWriter::new(fs::File::create(&hostile).unwrap());
    for (name, bytes) in entries {
        writer
            .start_file(
                name,
                SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
            )
            .unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
    let before = registry_json(&vault);
    let status = vault.status().unwrap();
    let error = vault.inspect_restore(&hostile).unwrap_err();
    assert!(!error.message.is_empty());
    assert_eq!(registry_json(&vault), before);
    assert_eq!(vault.status().unwrap().generation_id, status.generation_id);
    assert_eq!(
        vault.status().unwrap().mutation_sequence,
        status.mutation_sequence
    );
}
