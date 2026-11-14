use patrimoine_domain::*;

fn f(num: &str, den: &str) -> Fraction {
    Fraction {
        num: num.into(),
        den: den.into(),
    }
}
fn entity(id: &str, kind: &str) -> Entity {
    Entity {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: kind.into(),
        display_name: id.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: None,
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}
fn participation(id: &str, owner: &str, company: &str, share: Fraction) -> Participation {
    Participation {
        id: id.into(),
        revision_id: format!("{id}-r"),
        owner_entity_id: owner.into(),
        company_id: company.into(),
        legal_share: Some(share.clone()),
        economic_share: Some(share.clone()),
        voting_share: Some(share),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    }
}
fn fixture() -> RegistrySnapshot {
    RegistrySnapshot {
        entities: vec![
            entity("a", "person"),
            entity("outside", "person"),
            entity("c", "company"),
        ],
        assets: vec![Asset {
            id: "asset".into(),
            revision_id: "asset-r".into(),
            name: "Bien".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        }],
        memberships: vec![Membership {
            id: "member".into(),
            revision_id: "member-r".into(),
            person_id: "a".into(),
            valid_from: "2020-01-01".into(),
            valid_to: Some("2024-07-01".into()),
            relationship: String::new(),
        }],
        participations: vec![
            participation("pa", "a", "c", f("3", "5")),
            participation("px", "outside", "c", f("2", "5")),
        ],
        ownerships: vec![Ownership {
            id: "title".into(),
            revision_id: "title-r".into(),
            asset_id: "asset".into(),
            owner_entity_id: "c".into(),
            legal_share: Some(f("1", "1")),
            economic_share: Some(f("1", "1")),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        }],
        company_policies: vec![CompanyPolicy {
            id: "policy".into(),
            revision_id: "policy-r".into(),
            company_id: "c".into(),
            mode: "global_equity".into(),
            coverage_complete: true,
            liability_qualified: false,
            valid_from: "2020-01-01".into(),
            valid_to: None,
        }],
        ..Default::default()
    }
}

#[test]
fn analytical_income_looks_through_global_equity_without_requiring_a_valuation() {
    let s = fixture();
    let result = economic_asset_weight(&s, "asset", "family", None, "2024-06-30").unwrap();
    assert_eq!(result.weight, f("3", "5"));
    assert!(!result.unknown);
    for id in ["pa-r", "title-r", "member-r"] {
        assert!(result.source_ids.iter().any(|source| source == id));
    }
    assert!(s.valuations.is_empty());
}

#[test]
fn family_membership_end_is_exclusive_but_person_rights_survive_exit() {
    let s = fixture();
    let family = economic_asset_weight(&s, "asset", "family", None, "2024-07-01").unwrap();
    assert_eq!(family.weight, f("0", "1"));
    assert!(!family.unknown);
    let person = economic_asset_weight(&s, "asset", "person", Some("a"), "2024-07-01").unwrap();
    assert_eq!(person.weight, f("3", "5"));
    let whole = economic_asset_weight(&s, "asset", "property", None, "2024-07-01").unwrap();
    assert_eq!(whole.weight, f("1", "1"));
}

#[test]
fn nested_company_income_multiplies_exact_rights_and_preserves_unknowns() {
    let mut s = fixture();
    s.entities.push(entity("d", "company"));
    s.participations
        .push(participation("pc", "c", "d", f("1", "2")));
    s.participations
        .push(participation("po", "outside", "d", f("1", "2")));
    s.ownerships[0].owner_entity_id = "d".into();
    let result = economic_asset_weight(&s, "asset", "family", None, "2024-06-30").unwrap();
    assert_eq!(result.weight, f("3", "10"));
    assert!(!result.unknown);
    s.participations[0].economic_share = None;
    let unknown = economic_asset_weight(&s, "asset", "family", None, "2024-06-30").unwrap();
    assert_eq!(unknown.weight, f("0", "1"));
    assert!(unknown.unknown);
}

#[test]
fn income_context_and_dates_are_validated_without_silent_scope_fallback() {
    let s = fixture();
    assert!(economic_asset_weight(&s, "asset", "person", Some("c"), "2024-06-30").is_err());
    assert!(economic_asset_weight(&s, "asset", "property", Some("a"), "2024-06-30").is_err());
    assert!(economic_asset_weight(&s, "asset", "unknown", None, "2024-06-30").is_err());
    assert!(economic_asset_weight(&s, "missing", "family", None, "2024-06-30").is_err());
    assert!(economic_asset_weight(&s, "asset", "family", None, "2023-02-29").is_err());
}

#[test]
fn company_context_observes_its_own_economic_rights_without_attributing_them_to_shareholders() {
    let mut s = fixture();
    assert_eq!(
        economic_asset_weight(&s, "asset", "company", Some("c"), "2024-06-30")
            .unwrap()
            .weight,
        f("1", "1")
    );
    s.entities.push(entity("d", "company"));
    s.participations
        .push(participation("pc", "c", "d", f("1", "2")));
    s.participations
        .push(participation("po", "outside", "d", f("1", "2")));
    s.ownerships[0].owner_entity_id = "d".into();
    assert_eq!(
        economic_asset_weight(&s, "asset", "company", Some("c"), "2024-06-30")
            .unwrap()
            .weight,
        f("1", "2")
    );
    assert_eq!(
        economic_asset_weight(&s, "asset", "person", Some("a"), "2024-06-30")
            .unwrap()
            .weight,
        f("3", "10")
    );
    assert!(economic_asset_weight(&s, "asset", "company", Some("a"), "2024-06-30").is_err());
}
