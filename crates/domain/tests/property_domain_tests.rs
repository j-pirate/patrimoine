use patrimoine_domain::*;

fn ratio(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
    }
}

fn fixture() -> (RegistrySnapshot, PropertySnapshot) {
    let registry = RegistrySnapshot {
        knowledge_sequence: 12,
        entities: vec![Entity {
            id: "owner".into(),
            revision_id: "owner-r".into(),
            kind: "person".into(),
            display_name: "Bailleur".into(),
            first_name: None,
            last_name: None,
            birth_date: None,
            residence_country: Some("FR".into()),
            preferred_currency: "EUR".into(),
            activity: String::new(),
            notes: String::new(),
            archived: false,
        }],
        assets: vec![Asset {
            id: "asset".into(),
            revision_id: "asset-r".into(),
            name: "Immeuble".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        }],
        ownerships: vec![Ownership {
            id: "ownership".into(),
            revision_id: "ownership-r".into(),
            asset_id: "asset".into(),
            owner_entity_id: "owner".into(),
            legal_share: Some(ratio("1", "1")),
            economic_share: Some(ratio("1", "1")),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        }],
        valuations: vec![Valuation {
            id: "valuation".into(),
            revision_id: "valuation-r".into(),
            target_kind: "asset".into(),
            target_id: "asset".into(),
            amount: "200000".into(),
            currency: "EUR".into(),
            effective_on: "2024-03-01".into(),
            source: "Estimation de test".into(),
        }],
        profile_periods: vec![ProfilePeriod {
            id: "profile-period".into(),
            revision_id: "profile-period-r".into(),
            target_kind: "asset".into(),
            target_id: "asset".into(),
            profile_revision_id: "asset-r".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        }],
        ..Default::default()
    };
    let properties = PropertySnapshot {
        knowledge_sequence: 12,
        properties: vec![Property {
            id: "property".into(),
            revision_id: "property-r".into(),
            asset_id: "asset".into(),
            property_type: "building".into(),
            city: "Paris".into(),
            address: "Adresse synthétique".into(),
            surface_m2: Some("100.125".into()),
            status: "vacant".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            notes: String::new(),
        }],
        units: vec![RentalUnit {
            id: "unit".into(),
            revision_id: "unit-r".into(),
            property_asset_id: "asset".into(),
            reference: "A".into(),
            name: "Unité A".into(),
            surface_m2: Some("50.0625".into()),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            available_for_rent: true,
            notes: String::new(),
        }],
        events: vec![PropertyEvent {
            id: "acquisition".into(),
            revision_id: "acquisition-r".into(),
            asset_id: "asset".into(),
            kind: "acquisition".into(),
            effective_on: "2020-01-01".into(),
            price: Some("100000".into()),
            fees: Some("10000".into()),
            currency: "EUR".into(),
            notes: String::new(),
        }],
    };
    (registry, properties)
}

fn query(date: &str) -> PropertyReportRequest {
    PropertyReportRequest {
        asset_id: "asset".into(),
        date: date.into(),
        knowledge_sequence: Some(12),
    }
}

fn add_rate(registry: &mut RegistrySnapshot, id: &str, base: &str, value: &str, date: &str) {
    registry.exchange_rates.push(ExchangeRate {
        id: id.into(),
        revision_id: format!("{id}-r"),
        base_currency: base.into(),
        quote_currency: "EUR".into(),
        rate: value.into(),
        effective_on: date.into(),
        source: Some("Cours de test".into()),
        notes: String::new(),
    });
}

#[test]
fn costs_estimates_and_gains_are_separate_without_invented_debt() {
    let (registry, properties) = fixture();
    let r = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(r.acquisition_cost.as_deref(), Some("110000.00"));
    assert_eq!(r.exact_acquisition_cost, Some(ratio("110000", "1")));
    assert_eq!(r.estimated_value_eur.as_deref(), Some("200000.00"));
    assert_eq!(r.unrealized_gain_original.as_deref(), Some("90000.00"));
    assert_eq!(r.unrealized_gain_eur.as_deref(), Some("90000.00"));
    assert_eq!(r.debt_remaining_eur, None);
    assert_eq!(r.net_before_disposal_eur, None);
    assert_eq!(r.status, "vacant");
    assert_eq!(r.country.as_deref(), Some("FR"));
    assert_eq!(r.property.unwrap().surface_m2.as_deref(), Some("100.125"));
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "financing_not_inventoried"));
}

#[test]
fn historical_cost_fx_and_current_value_fx_are_not_a_current_conversion_of_gain() {
    let (mut registry, mut properties) = fixture();
    properties.events[0].price = Some("1000".into());
    properties.events[0].fees = Some("100".into());
    properties.events[0].currency = "TND".into();
    registry.valuations[0].amount = "1500".into();
    registry.valuations[0].currency = "TND".into();
    add_rate(&mut registry, "old-fx", "TND", "0.3", "2020-01-01");
    add_rate(&mut registry, "current-fx", "TND", "0.4", "2024-01-01");
    let r = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(r.acquisition_cost.as_deref(), Some("1100.000"));
    assert_eq!(r.acquisition_cost_eur.as_deref(), Some("330.00"));
    assert_eq!(r.estimated_value_eur.as_deref(), Some("600.00"));
    assert_eq!(r.unrealized_gain_original.as_deref(), Some("400.000"));
    assert_eq!(r.unrealized_gain_eur.as_deref(), Some("270.00"));
    assert_eq!(r.acquisition_conversions[0].date, "2020-01-01");
    assert_eq!(r.valuation_conversion.unwrap().date, "2024-06-30");
}

#[test]
fn differing_original_currencies_only_have_a_comparable_historic_eur_gain() {
    let (mut registry, properties) = fixture();
    registry.valuations[0].currency = "USD".into();
    add_rate(&mut registry, "usd-fx", "USD", "0.9", "2024-01-01");
    let r = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(r.unrealized_gain_original, None);
    assert_eq!(r.unrealized_gain_currency, None);
    assert_eq!(r.unrealized_gain_eur.as_deref(), Some("70000.00"));
}

#[test]
fn no_current_estimate_is_inferred_from_purchase_or_future_valuation() {
    let (mut registry, properties) = fixture();
    registry.valuations[0].effective_on = "2025-01-01".into();
    let r = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(r.acquisition_cost_eur.as_deref(), Some("110000.00"));
    assert_eq!(r.estimated_value_eur, None);
    assert_eq!(r.unrealized_gain_eur, None);
    assert!(r.warnings.iter().any(|w| w.code == "missing_valuation"));
}

#[test]
fn missing_cost_component_stays_unknown_while_explicit_zero_fees_are_known() {
    let (registry, mut properties) = fixture();
    properties.events[0].fees = None;
    let unknown = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(unknown.acquisition_cost, None);
    assert_eq!(unknown.unrealized_gain_eur, None);
    assert!(unknown.source_ids.contains(&"acquisition-r".into()));
    properties.events[0].fees = Some("0".into());
    let known = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(known.acquisition_cost.as_deref(), Some("100000.00"));
    assert_eq!(known.unrealized_gain_eur.as_deref(), Some("100000.00"));
}

#[test]
fn unknown_historical_cost_fx_does_not_destroy_original_cost_or_invent_eur_gain() {
    let (registry, mut properties) = fixture();
    properties.events[0].currency = "TND".into();
    let r = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(r.acquisition_cost.as_deref(), Some("110000.000"));
    assert_eq!(r.acquisition_cost_eur, None);
    assert_eq!(r.unrealized_gain_eur, None);
    assert_eq!(r.estimated_value_eur.as_deref(), Some("200000.00"));
    assert!(r.warnings.iter().any(|w| w.code == "missing_exchange_rate"));
}

#[test]
fn dated_property_details_and_units_use_exclusive_interval_end() {
    let (registry, mut properties) = fixture();
    properties.properties[0].valid_to = Some("2024-07-01".into());
    properties.units[0].valid_to = Some("2024-07-01".into());
    let mut next = properties.properties[0].clone();
    next.id = "next-detail".into();
    next.revision_id = "next-detail-r".into();
    next.valid_from = "2024-07-01".into();
    next.valid_to = None;
    next.status = "renovation".into();
    properties.properties.push(next);
    let before = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    let after = property_report(&registry, &properties, &query("2024-07-01")).unwrap();
    assert_eq!(before.status, "vacant");
    assert_eq!(before.units.len(), 1);
    assert_eq!(after.status, "renovation");
    assert_eq!(after.units.len(), 0);
}

#[test]
fn missing_descriptive_period_never_assigns_historical_country() {
    let (mut registry, properties) = fixture();
    registry.profile_periods.clear();
    let r = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(r.country, None);
    assert_eq!(r.label, "Immeuble");
    assert!(r.warnings.iter().any(|w| w.code == "undated_profile"));
}

#[test]
fn sold_property_keeps_sources_but_no_current_stock_or_unrealized_gain() {
    let (mut registry, mut properties) = fixture();
    registry.ownerships[0].valid_to = Some("2024-07-01".into());
    properties.events.push(PropertyEvent {
        id: "sale".into(),
        revision_id: "sale-r".into(),
        asset_id: "asset".into(),
        kind: "disposal".into(),
        effective_on: "2024-07-01".into(),
        price: Some("210000".into()),
        fees: Some("2000".into()),
        currency: "EUR".into(),
        notes: String::new(),
    });
    let before = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    let after = property_report(&registry, &properties, &query("2024-07-01")).unwrap();
    assert_eq!(before.status, "vacant");
    assert_eq!(before.estimated_value_eur.as_deref(), Some("200000.00"));
    assert_eq!(after.status, "sold");
    assert_eq!(after.estimated_value_eur, None);
    assert_eq!(after.unrealized_gain_eur, None);
    assert!(after.valuation.is_some());
    assert!(after.source_ids.contains(&"sale-r".into()));
    assert_eq!(after.acquisition_cost.as_deref(), Some("110000.00"));
}

#[test]
fn disposal_cannot_leave_active_or_planned_future_rights() {
    let (mut registry, mut properties) = fixture();
    let sale = PropertyEvent {
        id: "sale".into(),
        revision_id: "sale-r".into(),
        asset_id: "asset".into(),
        kind: "disposal".into(),
        effective_on: "2024-07-01".into(),
        price: None,
        fees: None,
        currency: "EUR".into(),
        notes: String::new(),
    };
    properties.events.push(sale);
    assert!(validate_properties(&registry, &properties).is_err());
    registry.ownerships[0].valid_to = Some("2024-07-01".into());
    assert!(validate_properties(&registry, &properties).is_ok());
    let mut future = registry.ownerships[0].clone();
    future.id = "future-right".into();
    future.revision_id = "future-right-r".into();
    future.valid_from = "2025-01-01".into();
    future.valid_to = None;
    registry.ownerships.push(future);
    assert!(validate_properties(&registry, &properties).is_err());
}

#[test]
fn acquisition_cannot_postdate_known_ownership_and_opening_tracking_cannot_invent_cost() {
    let (registry, mut properties) = fixture();
    properties.events[0].effective_on = "2020-01-02".into();
    assert!(validate_properties(&registry, &properties).is_err());
    properties.events[0].kind = "opening_tracking".into();
    assert!(validate_properties(&registry, &properties).is_err());
    properties.events[0].price = None;
    properties.events[0].fees = None;
    let report = property_report(&registry, &properties, &query("2024-06-30")).unwrap();
    assert_eq!(report.acquisition_cost, None);
}

#[test]
fn ambiguous_lifecycle_and_direct_sold_status_are_rejected() {
    let (registry, mut properties) = fixture();
    properties.properties[0].status = "sold".into();
    assert!(validate_properties(&registry, &properties).is_err());
    properties.properties[0].status = "vacant".into();
    let mut another = properties.events[0].clone();
    another.id = "other-acquisition".into();
    another.revision_id = "other-acquisition-r".into();
    properties.events.push(another);
    assert!(validate_properties(&registry, &properties).is_err());
}

#[test]
fn invalid_surface_price_overlap_reference_and_object_alias_are_rejected() {
    let (registry, original) = fixture();
    for invalid_surface in ["0", "-1", "1.00", "NaN"] {
        let mut properties = original.clone();
        properties.units[0].surface_m2 = Some(invalid_surface.into());
        assert!(validate_properties(&registry, &properties).is_err());
    }
    let mut properties = original.clone();
    properties.events[0].fees = Some("-1".into());
    assert!(validate_properties(&registry, &properties).is_err());
    properties = original.clone();
    let mut overlap = properties.properties[0].clone();
    overlap.id = "overlap".into();
    overlap.revision_id = "overlap-r".into();
    properties.properties.push(overlap);
    assert!(validate_properties(&registry, &properties).is_err());
    properties = original.clone();
    let mut duplicate = properties.units[0].clone();
    duplicate.id = "duplicate".into();
    duplicate.revision_id = "duplicate-r".into();
    properties.units.push(duplicate);
    assert!(validate_properties(&registry, &properties).is_err());
    properties = original;
    properties.units[0].id = "asset".into();
    assert!(validate_properties(&registry, &properties).is_err());
}

#[test]
fn stale_estimate_warns_without_changing_its_source_value() {
    let (mut registry, properties) = fixture();
    registry.valuations[0].effective_on = "2023-01-01".into();
    let r = property_report(&registry, &properties, &query("2024-01-02")).unwrap();
    assert_eq!(r.estimated_value_eur.as_deref(), Some("200000.00"));
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "stale_property_valuation"));
}

#[test]
fn report_rejects_mismatched_knowledge_and_impossible_date() {
    let (registry, mut properties) = fixture();
    properties.knowledge_sequence = 13;
    assert!(property_report(&registry, &properties, &query("2024-06-30")).is_err());
    properties.knowledge_sequence = 12;
    let mut request = query("2024-06-30");
    request.knowledge_sequence = Some(11);
    assert!(property_report(&registry, &properties, &request).is_err());
    assert!(property_report(&registry, &properties, &query("2023-02-29")).is_err());
}
