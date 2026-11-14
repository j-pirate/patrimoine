use patrimoine_domain::*;

fn fraction(n: u64, d: u64) -> Fraction {
    Fraction {
        num: n.to_string(),
        den: d.to_string(),
    }
}

fn person(id: &str) -> Entity {
    Entity {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: "person".into(),
        display_name: id.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}

fn fixture(currency: &str, amount: &str) -> RegistrySnapshot {
    let mut s = RegistrySnapshot {
        entities: vec![person("a"), person("outside")],
        ..Default::default()
    };
    s.memberships.push(Membership {
        id: "member".into(),
        revision_id: "member-r".into(),
        person_id: "a".into(),
        valid_from: "2020-01-01".into(),
        valid_to: None,
        relationship: String::new(),
    });
    s.assets.push(Asset {
        id: "home".into(),
        revision_id: "home-r".into(),
        name: "Maison".into(),
        country: "TN".into(),
        category: "real_estate".into(),
        notes: String::new(),
        archived: false,
    });
    s.ownerships.push(Ownership {
        id: "ownership".into(),
        revision_id: "ownership-r".into(),
        asset_id: "home".into(),
        owner_entity_id: "a".into(),
        legal_share: Some(fraction(1, 1)),
        economic_share: Some(fraction(1, 1)),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    });
    s.valuations.push(Valuation {
        id: "value-old".into(),
        revision_id: "value-old-r".into(),
        target_kind: "asset".into(),
        target_id: "home".into(),
        amount: amount.into(),
        currency: currency.into(),
        effective_on: "2024-01-01".into(),
        source: "Estimation de test".into(),
    });
    s
}

fn rate(s: &mut RegistrySnapshot, id: &str, effective_on: &str, value: &str) {
    s.exchange_rates.push(ExchangeRate {
        id: id.into(),
        revision_id: format!("{id}-r"),
        base_currency: "TND".into(),
        quote_currency: "EUR".into(),
        rate: value.into(),
        effective_on: effective_on.into(),
        source: Some("Cours de test".into()),
        notes: String::new(),
    });
}

fn request() -> VariationRequest {
    VariationRequest {
        context: "family".into(),
        person_id: None,
        from_date: "2024-01-01".into(),
        to_date: "2024-02-01".into(),
        knowledge_sequence: None,
    }
}

fn step<'a>(r: &'a VariationReport, code: &str) -> &'a VariationStep {
    r.steps.iter().find(|s| s.code == code).unwrap()
}

#[test]
fn rate_change_alone_is_separate_from_valuation() {
    let mut s = fixture("TND", "1000");
    rate(&mut s, "rate-old", "2024-01-01", "0.3");
    rate(&mut s, "rate-new", "2024-02-01", "0.31");
    let r = explain_variation(&s, &request()).unwrap();
    assert_eq!(r.opening.gross_eur, "300.00");
    assert_eq!(r.closing.gross_eur, "310.00");
    assert_eq!(step(&r, "valuation").amount_eur.as_deref(), Some("0.00"));
    assert_eq!(step(&r, "fx").amount_eur.as_deref(), Some("10.00"));
    assert_eq!(r.exact_delta, fraction(10, 1));
    assert_eq!(r.exact_explained, fraction(10, 1));
    assert_eq!(r.exact_residual, Some(fraction(0, 1)));
    assert_eq!(r.status, "complete");
    assert_eq!(r.opening.date, "2024-01-01");
    assert_eq!(r.closing.date, "2024-02-01");
    assert!(step(&r, "fx").source_ids.contains(&"rate-old".into()));
    assert!(step(&r, "fx").source_ids.contains(&"rate-new".into()));
}

#[test]
fn price_change_precedes_fx_and_the_cross_effect_belongs_to_fx() {
    let mut s = fixture("TND", "1000");
    rate(&mut s, "rate-old", "2024-01-01", "0.3");
    rate(&mut s, "rate-new", "2024-02-01", "0.4");
    let mut value = s.valuations[0].clone();
    value.id = "value-new".into();
    value.revision_id = "value-new-r".into();
    value.amount = "1200".into();
    value.effective_on = "2024-02-01".into();
    s.valuations.push(value);
    let r = explain_variation(&s, &request()).unwrap();
    assert_eq!(r.delta_eur, "180.00");
    assert_eq!(step(&r, "valuation").amount_eur.as_deref(), Some("60.00"));
    assert_eq!(step(&r, "fx").amount_eur.as_deref(), Some("120.00"));
    assert_eq!(step(&r, "valuation").dates.fx_date, "2024-01-01");
    assert_eq!(step(&r, "fx").dates.valuation_date, "2024-02-01");
    assert_eq!(r.exact_explained, r.exact_delta);
}

#[test]
fn member_entry_changes_scope_without_becoming_a_price_gain() {
    let mut s = fixture("EUR", "1000");
    s.memberships[0].valid_from = "2024-02-01".into();
    let r = explain_variation(&s, &request()).unwrap();
    assert_eq!(r.opening.gross_eur, "0.00");
    assert_eq!(r.closing.gross_eur, "1000.00");
    assert_eq!(
        step(&r, "scope_method").amount_eur.as_deref(),
        Some("1000.00")
    );
    assert_eq!(step(&r, "rights").amount_eur.as_deref(), Some("0.00"));
    assert_eq!(step(&r, "valuation").amount_eur.as_deref(), Some("0.00"));
    assert_eq!(step(&r, "fx").amount_eur.as_deref(), Some("0.00"));
}

#[test]
fn changing_known_rights_is_an_explicit_bridge_step() {
    let mut s = fixture("EUR", "1000");
    s.ownerships[0].economic_share = Some(fraction(1, 2));
    s.ownerships[0].legal_share = Some(fraction(1, 2));
    s.ownerships[0].valid_to = Some("2024-02-01".into());
    let mut outside = s.ownerships[0].clone();
    outside.id = "outside-old".into();
    outside.revision_id = "outside-old-r".into();
    outside.owner_entity_id = "outside".into();
    s.ownerships.push(outside);
    let mut current = s.ownerships[0].clone();
    current.id = "ownership-new".into();
    current.revision_id = "ownership-new-r".into();
    current.economic_share = Some(fraction(1, 1));
    current.legal_share = Some(fraction(1, 1));
    current.valid_from = "2024-02-01".into();
    current.valid_to = None;
    s.ownerships.push(current);
    let r = explain_variation(&s, &request()).unwrap();
    assert_eq!(r.delta_eur, "500.00");
    assert_eq!(step(&r, "rights").amount_eur.as_deref(), Some("500.00"));
    assert_eq!(step(&r, "valuation").amount_eur.as_deref(), Some("0.00"));
    assert_eq!(r.exact_residual, Some(fraction(0, 1)));
}

#[test]
fn missing_reference_remains_unknown_and_leaves_an_explicit_residual() {
    let mut s = fixture("EUR", "1000");
    s.valuations[0].effective_on = "2024-02-01".into();
    let r = explain_variation(&s, &request()).unwrap();
    assert_eq!(r.opening.gross_eur, "0.00");
    assert_eq!(r.closing.gross_eur, "1000.00");
    assert_eq!(step(&r, "valuation").amount_eur, None);
    assert_eq!(step(&r, "valuation").exact_amount, None);
    assert_eq!(r.exact_explained, fraction(0, 1));
    assert_eq!(r.exact_residual, Some(fraction(1000, 1)));
    assert_eq!(r.status, "partial");
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "variation_partial_subtotal"));
}

#[test]
fn exact_fractional_change_reconciles_before_display_rounding() {
    let mut s = fixture("EUR", "1");
    s.ownerships[0].economic_share = Some(fraction(1, 3));
    s.ownerships[0].legal_share = Some(fraction(1, 3));
    let mut outside = s.ownerships[0].clone();
    outside.id = "outside-right".into();
    outside.revision_id = "outside-right-r".into();
    outside.owner_entity_id = "outside".into();
    outside.economic_share = Some(fraction(2, 3));
    outside.legal_share = Some(fraction(2, 3));
    s.ownerships.push(outside);
    let mut current = s.valuations[0].clone();
    current.id = "new-value".into();
    current.revision_id = "new-value-r".into();
    current.amount = "2".into();
    current.effective_on = "2024-02-01".into();
    s.valuations.push(current);
    let r = explain_variation(&s, &request()).unwrap();
    assert_eq!(r.exact_delta, fraction(1, 3));
    assert_eq!(r.exact_explained, fraction(1, 3));
    assert_eq!(step(&r, "valuation").exact_amount, Some(fraction(1, 3)));
    assert_eq!(r.exact_residual, Some(fraction(0, 1)));
    assert_eq!(r.delta_eur, "0.33");
}

#[test]
fn variation_checks_dates_and_matches_the_knowledge_snapshot() {
    let s = fixture("EUR", "1000");
    let mut q = request();
    q.from_date = "2024-02-02".into();
    assert!(explain_variation(&s, &q).is_err());
    q = request();
    q.knowledge_sequence = Some(42);
    assert!(explain_variation(&s, &q).is_err());
}
