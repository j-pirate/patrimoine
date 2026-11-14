use patrimoine_domain::*;

fn person(id: &str, revision: &str, name: &str, country: &str) -> Entity {
    Entity {
        id: id.into(),
        revision_id: revision.into(),
        kind: "person".into(),
        display_name: name.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: Some(country.into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}

fn period(id: &str, revision: &str, from: &str, to: Option<&str>) -> ProfilePeriod {
    ProfilePeriod {
        id: id.into(),
        revision_id: format!("{id}-r"),
        target_kind: "entity".into(),
        target_id: "a".into(),
        profile_revision_id: revision.into(),
        valid_from: from.into(),
        valid_to: to.map(String::from),
    }
}

fn fixture() -> RegistrySnapshot {
    let old = person("a", "a-old", "Ancien nom", "FR");
    let current = person("a", "a-current", "Nom corrigé", "TN");
    RegistrySnapshot {
        entities: vec![current.clone()],
        profile_revisions: vec![old, current],
        profile_periods: vec![
            period("before", "a-old", "2020-01-01", Some("2024-07-01")),
            period("after", "a-current", "2024-07-01", None),
        ],
        ..Default::default()
    }
}

#[test]
fn dated_profiles_use_exact_revisions_and_exclusive_end_without_mutating_sources() {
    let s = fixture();
    let before = apply_profile_periods(&s, "2024-06-30").unwrap();
    assert_eq!(before.entities[0].display_name, "Ancien nom");
    assert_eq!(before.entities[0].residence_country.as_deref(), Some("FR"));
    assert_eq!(before.entities[0].revision_id, "a-old");
    let after = apply_profile_periods(&s, "2024-07-01").unwrap();
    assert_eq!(after.entities[0].display_name, "Nom corrigé");
    assert_eq!(after.entities[0].residence_country.as_deref(), Some("TN"));
    assert_eq!(s.entities[0].revision_id, "a-current");
    assert_eq!(s.profile_revisions.len(), 2);
    // An unrecorded period never implies the oldest stored descriptive version.
    assert_eq!(
        apply_profile_periods(&s, "2019-01-01").unwrap().entities[0].revision_id,
        "a-current"
    );
}

#[test]
fn wrong_identity_missing_revision_and_overlapping_periods_are_rejected() {
    let mut s = fixture();
    s.entities.push(person("b", "b-current", "B", "CH"));
    s.profile_periods[0].profile_revision_id = "b-current".into();
    assert!(validate_registry(&s).is_err());
    s.profile_periods[0].profile_revision_id = "missing".into();
    assert!(validate_registry(&s).is_err());
    s = fixture();
    s.profile_periods[0].valid_to = Some("2024-07-02".into());
    assert!(validate_registry(&s).is_err());
    s = fixture();
    s.profile_periods[0].target_kind = "asset".into();
    assert!(validate_registry(&s).is_err());
}

#[test]
fn voided_profiles_keep_history_but_cannot_receive_active_periods() {
    let mut s = fixture();
    s.entities.clear();
    s.profile_periods.clear();
    assert!(validate_registry(&s).is_ok());
    s.profile_periods
        .push(period("orphan", "a-old", "2020-01-01", None));
    assert!(validate_registry(&s).is_err());
    s.profile_periods.clear();
    s.asset_profile_revisions.push(Asset {
        id: "asset".into(),
        revision_id: "asset-old".into(),
        name: "Actif annulé".into(),
        country: "FR".into(),
        category: "other".into(),
        notes: String::new(),
        archived: false,
    });
    assert!(validate_registry(&s).is_ok());
}

#[test]
fn historical_profiles_cannot_reuse_revisions_or_change_identity_kind() {
    let mut s = fixture();
    s.profile_revisions[0].kind = "company".into();
    assert!(validate_registry(&s).is_err());
    s.entities.clear();
    s.profile_periods.clear();
    assert!(validate_registry(&s).is_err());
    s = fixture();
    s.profile_revisions[0].revision_id = "a-current".into();
    assert!(validate_registry(&s).is_err());
    s = fixture();
    s.profile_revisions[1].display_name = "Contenu altéré".into();
    assert!(validate_registry(&s).is_err());
    s = fixture();
    s.asset_profile_revisions.push(Asset {
        id: "a".into(),
        revision_id: "asset-old".into(),
        name: "Collision".into(),
        country: "FR".into(),
        category: "other".into(),
        notes: String::new(),
        archived: false,
    });
    assert!(validate_registry(&s).is_err());
}
