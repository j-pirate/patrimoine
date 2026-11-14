use patrimoine_application::{PropertyService, RegistryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn ratio() -> Option<Fraction> {
    Some(Fraction {
        num: "1".into(),
        den: "1".into(),
    })
}
fn fixture() -> (TempDir, Vault, Asset, Ownership) {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("vault")).unwrap();
    let person = vault
        .save_entity(SaveEntityInput {
            id: None,
            expected_revision_id: None,
            reason: "Test".into(),
            kind: "person".into(),
            display_name: "Propriétaire".into(),
            first_name: None,
            last_name: None,
            birth_date: None,
            residence_country: Some("FR".into()),
            preferred_currency: "EUR".into(),
            activity: String::new(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    vault
        .save_membership(SaveMembershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Test".into(),
            person_id: person.id.clone(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            relationship: String::new(),
        })
        .unwrap();
    let asset = vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Test".into(),
            name: "Maison".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    let ownership = vault
        .save_ownership(SaveOwnershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Test".into(),
            asset_id: asset.id.clone(),
            owner_entity_id: person.id,
            legal_share: ratio(),
            economic_share: ratio(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    (temp, vault, asset, ownership)
}
fn property(asset: &Asset) -> SavePropertyInput {
    SavePropertyInput {
        id: None,
        expected_revision_id: None,
        reason: "Fiche test".into(),
        asset_id: asset.id.clone(),
        property_type: "house".into(),
        city: "Lyon".into(),
        address: "Adresse synthétique".into(),
        surface_m2: Some(" 90,50 ".into()),
        status: "vacant".into(),
        valid_from: "2020-01-01".into(),
        valid_to: None,
        notes: String::new(),
    }
}
fn event(asset: &Asset, kind: &str) -> SavePropertyEventInput {
    SavePropertyEventInput {
        id: None,
        expected_revision_id: None,
        reason: "Coût historique".into(),
        asset_id: asset.id.clone(),
        kind: kind.into(),
        effective_on: "2020-01-01".into(),
        price: Some("100000".into()),
        fees: Some("10000".into()),
        currency: "EUR".into(),
        notes: String::new(),
    }
}
fn dispose(asset: &Asset, sequence: u64) -> DisposePropertyInput {
    DisposePropertyInput {
        asset_id: asset.id.clone(),
        effective_on: "2026-07-01".into(),
        price: Some("160000".into()),
        fees: Some("1000".into()),
        currency: "EUR".into(),
        reason: "Cession synthétique".into(),
        expected_sequence: sequence,
    }
}
fn canonical(value: impl serde::Serialize) -> Value {
    serde_json::to_value(value).unwrap()
}

#[test]
fn property_cost_estimate_and_units_persist_without_new_consolidated_assets() {
    let (_temp, mut vault, asset, _) = fixture();
    let saved = vault.save_property(property(&asset)).unwrap();
    assert_eq!(saved.surface_m2.as_deref(), Some("90.5"));
    vault
        .save_property_event(event(&asset, "acquisition"))
        .unwrap();
    for reference in ["A", "B", "C"] {
        vault
            .save_rental_unit(SaveRentalUnitInput {
                id: None,
                expected_revision_id: None,
                reason: "Unité technique".into(),
                property_asset_id: asset.id.clone(),
                reference: reference.into(),
                name: format!("Lot {reference}"),
                surface_m2: Some("30".into()),
                valid_from: "2020-01-01".into(),
                valid_to: None,
                available_for_rent: true,
                notes: String::new(),
            })
            .unwrap();
    }
    let no_estimate = vault
        .get_property_report(PropertyReportRequest {
            asset_id: asset.id.clone(),
            date: "2026-01-01".into(),
            knowledge_sequence: None,
        })
        .unwrap();
    assert_eq!(no_estimate.acquisition_cost.as_deref(), Some("110000.00"));
    assert!(no_estimate.estimated_value_eur.is_none());
    vault
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Estimation".into(),
            target_kind: "asset".into(),
            target_id: asset.id.clone(),
            amount: "150000".into(),
            currency: "EUR".into(),
            effective_on: "2026-01-01".into(),
            source: "Test".into(),
        })
        .unwrap();
    let report = vault
        .get_property_report(PropertyReportRequest {
            asset_id: asset.id,
            date: "2026-01-01".into(),
            knowledge_sequence: None,
        })
        .unwrap();
    assert_eq!(report.estimated_value_eur.as_deref(), Some("150000.00"));
    assert_eq!(report.unrealized_gain_eur.as_deref(), Some("40000.00"));
    assert!(report.debt_remaining_eur.is_none());
    assert_eq!(report.units.len(), 3);
    assert_eq!(vault.list_registry().unwrap().assets.len(), 1);
    assert_eq!(
        vault
            .get_consolidation(ConsolidationRequest {
                context: "family".into(),
                person_id: None,
                date: "2026-01-01".into(),
                knowledge_sequence: None
            })
            .unwrap()
            .gross_eur,
        "150000.00"
    );
}

#[test]
fn invalid_property_conflicts_and_immutable_parent_roll_back_every_trace() {
    let (_temp, mut vault, asset, _) = fixture();
    let p = vault.save_property(property(&asset)).unwrap();
    let other = vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Autre identité".into(),
            name: "Autre bien".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    let baseline = canonical(vault.list_properties_at(None).unwrap());
    let history = canonical(vault.list_registry_history().unwrap());
    let seq = vault.status().unwrap().mutation_sequence;
    assert!(vault.save_property(property(&asset)).is_err());
    let mut bad = property(&asset);
    bad.id = Some(p.id.clone());
    bad.expected_revision_id = Some(p.revision_id.clone());
    bad.status = "sold".into();
    assert!(vault.save_property(bad).is_err());
    let mut stale = property(&asset);
    stale.id = Some(p.id.clone());
    stale.expected_revision_id = Some("00000000-0000-4000-8000-000000000000".into());
    assert!(vault.save_property(stale).is_err());
    let mut moved = property(&other);
    moved.id = Some(p.id.clone());
    moved.expected_revision_id = Some(p.revision_id.clone());
    assert!(vault.save_property(moved).is_err());
    assert_eq!(canonical(vault.list_properties_at(None).unwrap()), baseline);
    assert_eq!(canonical(vault.list_registry_history().unwrap()), history);
    assert_eq!(vault.status().unwrap().mutation_sequence, seq);
    assert!(vault
        .save_property_event(event(&asset, "disposal"))
        .is_err());
}

#[test]
fn full_disposal_is_one_change_and_closes_rights_without_erasing_earlier_knowledge() {
    let (temp, mut vault, asset, ownership) = fixture();
    vault.save_property(property(&asset)).unwrap();
    vault
        .save_property_event(event(&asset, "acquisition"))
        .unwrap();
    let before = vault.status().unwrap().mutation_sequence;
    let result = vault.dispose_property(dispose(&asset, before)).unwrap();
    assert_eq!(result.knowledge_sequence, before + 1);
    assert_eq!(
        result
            .events
            .iter()
            .filter(|e| e.kind == "disposal")
            .count(),
        1
    );
    let rights = vault.list_registry().unwrap().ownerships;
    assert_eq!(rights[0].id, ownership.id);
    assert_eq!(rights[0].valid_to.as_deref(), Some("2026-07-01"));
    assert_eq!(
        vault.list_registry_at(Some(before)).unwrap().ownerships[0].valid_to,
        None
    );
    let history = vault.list_registry_history().unwrap();
    assert_eq!(
        history
            .events
            .iter()
            .filter(|e| e.sequence == before + 1)
            .count(),
        2
    );
    assert!(vault.dispose_property(dispose(&asset, before)).is_err());
    let prior = canonical(vault.list_registry_at(Some(before)).unwrap());
    let mut reopen = rights[0].clone();
    reopen.valid_to = None;
    assert!(vault
        .save_ownership(SaveOwnershipInput {
            id: Some(reopen.id),
            expected_revision_id: Some(reopen.revision_id),
            reason: "Réouverture interdite".into(),
            asset_id: reopen.asset_id,
            owner_entity_id: reopen.owner_entity_id,
            legal_share: reopen.legal_share,
            economic_share: reopen.economic_share,
            valid_from: reopen.valid_from,
            valid_to: None
        })
        .is_err());
    assert_eq!(vault.status().unwrap().mutation_sequence, before + 1);
    drop(vault);
    let reopened = Vault::open(temp.path().join("vault")).unwrap();
    assert_eq!(
        canonical(reopened.list_registry_at(Some(before)).unwrap()),
        prior
    );
    assert_eq!(reopened.list_properties_at(None).unwrap().events.len(), 2);
}

#[test]
fn disposal_on_first_ownership_day_creates_an_exact_tombstone() {
    let (temp, mut vault, asset, old) = fixture();
    vault.save_property(property(&asset)).unwrap();
    let mut opening = event(&asset, "opening_tracking");
    opening.effective_on = "2019-12-31".into();
    opening.price = None;
    opening.fees = None;
    vault.save_property_event(opening).unwrap();
    let before = vault.status().unwrap().mutation_sequence;
    let mut request = dispose(&asset, before);
    request.effective_on = "2020-01-01".into();
    vault.dispose_property(request).unwrap();
    assert!(vault.list_registry().unwrap().ownerships.is_empty());
    assert_eq!(
        vault.list_registry_at(Some(before)).unwrap().ownerships[0].revision_id,
        old.revision_id
    );
    drop(vault);
    assert!(Vault::open(temp.path().join("vault")).is_ok());
}

#[test]
fn properties_and_lifecycle_versions_roundtrip_with_backup() {
    let (temp, mut vault, asset, _) = fixture();
    let p = vault.save_property(property(&asset)).unwrap();
    vault
        .save_property_event(event(&asset, "acquisition"))
        .unwrap();
    let old = vault.status().unwrap().mutation_sequence;
    let mut corrected = property(&asset);
    corrected.id = Some(p.id);
    corrected.expected_revision_id = Some(p.revision_id);
    corrected.city = "Marseille".into();
    vault.save_property(corrected).unwrap();
    let baseline = canonical(vault.list_properties_at(None).unwrap());
    let old_state = canonical(vault.list_properties_at(Some(old)).unwrap());
    let zip = temp.path().join("property.zip");
    vault.create_backup(&zip).unwrap();
    let bytes = fs::read(&zip).unwrap();
    let mut target = Vault::open(temp.path().join("target")).unwrap();
    let inspected = target.inspect_restore(&zip).unwrap();
    assert_eq!(inspected.schema_version, 6);
    target
        .activate_restore(&inspected.token, true, None)
        .unwrap();
    assert_eq!(
        canonical(target.list_properties_at(None).unwrap()),
        baseline
    );
    assert_eq!(
        canonical(target.list_properties_at(Some(old)).unwrap()),
        old_state
    );
    assert_eq!(fs::read(zip).unwrap(), bytes);
}

fn correction(event: &PropertyEvent, date: &str) -> SavePropertyEventInput {
    SavePropertyEventInput {
        id: Some(event.id.clone()),
        expected_revision_id: Some(event.revision_id.clone()),
        reason: "Correction motivée de la cession".into(),
        asset_id: event.asset_id.clone(),
        kind: "disposal".into(),
        effective_on: date.into(),
        price: Some("170000,50".into()),
        fees: Some("2000".into()),
        currency: "EUR".into(),
        notes: "Acte de cession corrigé".into(),
    }
}

#[test]
fn disposal_date_can_move_earlier_and_later_in_one_change_without_overwriting_old_knowledge() {
    let (temp, mut vault, asset, _) = fixture();
    vault.save_property(property(&asset)).unwrap();
    vault
        .save_property_event(event(&asset, "acquisition"))
        .unwrap();
    let sequence = vault.status().unwrap().mutation_sequence;
    let sold = vault.dispose_property(dispose(&asset, sequence)).unwrap();
    let sale = sold.events.iter().find(|e| e.kind == "disposal").unwrap();
    let original_k = sold.knowledge_sequence;
    let earlier = vault
        .save_property_event(correction(sale, "2026-06-01"))
        .unwrap();
    assert_eq!(earlier.id, sale.id);
    assert_ne!(earlier.revision_id, sale.revision_id);
    assert_eq!(earlier.price.as_deref(), Some("170000.5"));
    assert_eq!(vault.status().unwrap().mutation_sequence, original_k + 1);
    assert_eq!(
        vault.list_registry().unwrap().ownerships[0]
            .valid_to
            .as_deref(),
        Some("2026-06-01")
    );
    assert_eq!(
        vault.list_registry_at(Some(original_k)).unwrap().ownerships[0]
            .valid_to
            .as_deref(),
        Some("2026-07-01")
    );
    assert_eq!(
        vault
            .list_properties_at(Some(original_k))
            .unwrap()
            .events
            .iter()
            .find(|e| e.kind == "disposal")
            .unwrap()
            .price
            .as_deref(),
        Some("160000")
    );
    let earlier_k = vault.status().unwrap().mutation_sequence;
    let later = vault
        .save_property_event(correction(&earlier, "2026-09-01"))
        .unwrap();
    assert_eq!(later.effective_on, "2026-09-01");
    assert_eq!(vault.status().unwrap().mutation_sequence, earlier_k + 1);
    assert_eq!(
        vault.list_registry().unwrap().ownerships[0]
            .valid_to
            .as_deref(),
        Some("2026-09-01")
    );
    assert_eq!(
        vault.list_registry_at(Some(earlier_k)).unwrap().ownerships[0]
            .valid_to
            .as_deref(),
        Some("2026-06-01")
    );
    assert!(vault
        .save_property_event(correction(&earlier, "2026-08-01"))
        .is_err());
    assert_eq!(vault.status().unwrap().mutation_sequence, earlier_k + 1);
    drop(vault);
    assert!(Vault::open(temp.path().join("vault")).is_ok());
}

#[test]
fn correcting_disposal_to_first_right_day_and_back_preserves_tombstone_content() {
    let (temp, mut vault, asset, _) = fixture();
    vault.save_property(property(&asset)).unwrap();
    let mut opening = event(&asset, "opening_tracking");
    opening.effective_on = "2019-12-31".into();
    opening.price = None;
    opening.fees = None;
    vault.save_property_event(opening).unwrap();
    let seq = vault.status().unwrap().mutation_sequence;
    let sold = vault.dispose_property(dispose(&asset, seq)).unwrap();
    let sale = sold.events.iter().find(|e| e.kind == "disposal").unwrap();
    let first = vault
        .save_property_event(correction(sale, "2020-01-01"))
        .unwrap();
    let first_k = vault.status().unwrap().mutation_sequence;
    assert!(vault.list_registry().unwrap().ownerships.is_empty());
    vault
        .save_property_event(correction(&first, "2026-08-01"))
        .unwrap();
    assert_eq!(
        vault.list_registry().unwrap().ownerships[0]
            .valid_to
            .as_deref(),
        Some("2026-08-01")
    );
    assert!(vault
        .list_registry_at(Some(first_k))
        .unwrap()
        .ownerships
        .is_empty());
    drop(vault);
    assert!(Vault::open(temp.path().join("vault")).is_ok());
}

#[test]
fn disposal_correction_rejects_source_end_overrun_and_separately_changed_rights_atomically() {
    let (_temp, mut vault, asset, ownership) = fixture();
    vault.save_property(property(&asset)).unwrap();
    vault
        .save_property_event(event(&asset, "acquisition"))
        .unwrap();
    vault
        .save_ownership(SaveOwnershipInput {
            id: Some(ownership.id.clone()),
            expected_revision_id: Some(ownership.revision_id),
            reason: "Fin prévue".into(),
            asset_id: asset.id.clone(),
            owner_entity_id: ownership.owner_entity_id,
            legal_share: ratio(),
            economic_share: ratio(),
            valid_from: ownership.valid_from,
            valid_to: Some("2026-08-01".into()),
        })
        .unwrap();
    let seq = vault.status().unwrap().mutation_sequence;
    let sold = vault.dispose_property(dispose(&asset, seq)).unwrap();
    let sale = sold.events.iter().find(|e| e.kind == "disposal").unwrap();
    assert!(vault
        .save_property_event(correction(sale, "2026-08-02"))
        .is_err());
    assert_eq!(vault.status().unwrap().mutation_sequence, seq + 1);
    let current = vault.list_registry().unwrap().ownerships.remove(0);
    vault
        .save_ownership(SaveOwnershipInput {
            id: Some(current.id),
            expected_revision_id: Some(current.revision_id),
            reason: "Droit historique corrigé séparément".into(),
            asset_id: current.asset_id,
            owner_entity_id: current.owner_entity_id,
            legal_share: current.legal_share,
            economic_share: Some(Fraction {
                num: "1".into(),
                den: "2".into(),
            }),
            valid_from: current.valid_from,
            valid_to: current.valid_to,
        })
        .unwrap();
    let registry = canonical(vault.list_registry().unwrap());
    let property = canonical(vault.list_properties_at(None).unwrap());
    let history = canonical(vault.list_registry_history().unwrap());
    assert!(vault
        .save_property_event(correction(sale, "2026-06-01"))
        .is_err());
    assert_eq!(canonical(vault.list_registry().unwrap()), registry);
    assert_eq!(canonical(vault.list_properties_at(None).unwrap()), property);
    assert_eq!(canonical(vault.list_registry_history().unwrap()), history);
}

#[test]
fn disposal_price_correction_keeps_rights_and_is_reproduced_after_restore() {
    let (temp, mut vault, asset, _) = fixture();
    vault.save_property(property(&asset)).unwrap();
    vault
        .save_property_event(event(&asset, "acquisition"))
        .unwrap();
    let before = vault.status().unwrap().mutation_sequence;
    let sold = vault.dispose_property(dispose(&asset, before)).unwrap();
    let sale = sold.events.iter().find(|e| e.kind == "disposal").unwrap();
    let rights = canonical(vault.list_registry().unwrap().ownerships);
    let changed = vault
        .save_property_event(correction(sale, &sale.effective_on))
        .unwrap();
    assert_eq!(canonical(vault.list_registry().unwrap().ownerships), rights);
    assert_eq!(changed.fees.as_deref(), Some("2000"));
    let zip = temp.path().join("corrected-sale.zip");
    vault.create_backup(&zip).unwrap();
    let mut target = Vault::open(temp.path().join("restored")).unwrap();
    let inspect = target.inspect_restore(&zip).unwrap();
    target.activate_restore(&inspect.token, true, None).unwrap();
    assert_eq!(
        target
            .list_properties_at(None)
            .unwrap()
            .events
            .iter()
            .find(|e| e.kind == "disposal")
            .unwrap()
            .price
            .as_deref(),
        Some("170000.5")
    );
    assert_eq!(
        target
            .list_properties_at(Some(sold.knowledge_sequence))
            .unwrap()
            .events
            .iter()
            .find(|e| e.kind == "disposal")
            .unwrap()
            .price
            .as_deref(),
        Some("160000")
    );
}
