use patrimoine_domain::*;

fn f(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
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
fn fixture() -> (RegistrySnapshot, PropertySnapshot, RentalSnapshot) {
    let registry = RegistrySnapshot {
        knowledge_sequence: 1,
        entities: vec![person("owner"), person("tenant"), person("buyer")],
        memberships: vec![Membership {
            id: "member".into(),
            revision_id: "member-r".into(),
            person_id: "owner".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            relationship: String::new(),
        }],
        assets: vec![Asset {
            id: "asset".into(),
            revision_id: "asset-r".into(),
            name: "Bien".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        }],
        ownerships: vec![Ownership {
            id: "own".into(),
            revision_id: "own-r".into(),
            asset_id: "asset".into(),
            owner_entity_id: "owner".into(),
            legal_share: Some(f("1", "1")),
            economic_share: Some(f("1", "1")),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        }],
        valuations: vec![Valuation {
            id: "value".into(),
            revision_id: "value-r".into(),
            target_kind: "asset".into(),
            target_id: "asset".into(),
            amount: "200000".into(),
            currency: "EUR".into(),
            effective_on: "2020-01-01".into(),
            source: "Test".into(),
        }],
        ..Default::default()
    };
    let properties = PropertySnapshot {
        knowledge_sequence: 1,
        properties: vec![Property {
            id: "property".into(),
            revision_id: "property-r".into(),
            asset_id: "asset".into(),
            property_type: "building".into(),
            city: "Paris".into(),
            address: "Test".into(),
            surface_m2: None,
            status: "rented".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            notes: String::new(),
        }],
        units: vec![RentalUnit {
            id: "unit".into(),
            revision_id: "unit-r".into(),
            property_asset_id: "asset".into(),
            reference: "A".into(),
            name: "Unité".into(),
            surface_m2: None,
            valid_from: "2020-01-01".into(),
            valid_to: None,
            available_for_rent: true,
            notes: String::new(),
        }],
        events: vec![],
    };
    let terms = LeaseTerms {
        id: "terms".into(),
        revision_id: "terms-r".into(),
        lease_id: "lease".into(),
        rent: "1000".into(),
        charges: "0".into(),
        currency: "EUR".into(),
        frequency: "monthly".into(),
        due_day: 5,
        deposit_required: "2000".into(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: String::new(),
    };
    let rentals = RentalSnapshot {
        knowledge_sequence: 1,
        leases: vec![Lease {
            id: "lease".into(),
            revision_id: "lease-r".into(),
            unit_id: "unit".into(),
            tenant_entity_id: "tenant".into(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            notes: String::new(),
        }],
        lease_terms: vec![terms.clone()],
        lease_term_revisions: vec![terms],
        ..Default::default()
    };
    (registry, properties, rentals)
}
fn generate(
    r: &RegistrySnapshot,
    p: &PropertySnapshot,
    s: &mut RentalSnapshot,
    from: &str,
    to: &str,
) {
    let proposals = generate_rent_dues(
        r,
        p,
        s,
        &GenerateRentDuesInput {
            lease_id: "lease".into(),
            from_date: from.into(),
            to_date: to.into(),
            expected_sequence: 1,
            reason: "Test".into(),
        },
    )
    .unwrap();
    for (i, d) in proposals.into_iter().enumerate() {
        s.dues.push(RentDue {
            id: format!("due-{i}"),
            revision_id: format!("due-{i}-r"),
            lease_id: d.lease_id,
            terms_revision_id: d.terms_revision_id,
            period_start: d.period_start,
            period_end: d.period_end,
            due_on: d.due_on,
            rent: d.rent,
            charges: d.charges,
            currency: d.currency,
            generation_key: d.generation_key,
            notes: d.notes,
        });
    }
}
fn pay(s: &mut RentalSnapshot, id: &str, amount: &str, day: &str) {
    s.payments.push(RentPayment {
        id: id.into(),
        revision_id: format!("{id}-r"),
        lease_id: "lease".into(),
        kind: "receipt".into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: day.into(),
        original_payment_id: None,
        recipient_entity_id: "owner".into(),
        reference: String::new(),
        unlinked_reason: "Pas encore rapproché".into(),
    });
}
fn allocate(s: &mut RentalSnapshot, id: &str, payment: &str, due: &str, amount: &str, day: &str) {
    s.allocations.push(RentAllocation {
        id: id.into(),
        revision_id: format!("{id}-r"),
        payment_id: payment.into(),
        due_id: due.into(),
        component: "rent".into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: day.into(),
    });
}
fn q(from: &str, to: &str, day: &str) -> RentalReportRequest {
    RentalReportRequest {
        country: None,
        context: "property".into(),
        person_id: None,
        asset_id: Some("asset".into()),
        from_date: from.into(),
        to_date: to.into(),
        date: day.into(),
        knowledge_sequence: Some(1),
    }
}

#[test]
fn two_partial_receipts_leave_exact_three_hundred_and_preserve_sources() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    pay(&mut s, "p1", "400", "2024-03-05");
    pay(&mut s, "p2", "300", "2024-03-20");
    allocate(&mut s, "a1", "p1", "due-0", "400", "2024-03-05");
    allocate(&mut s, "a2", "p2", "due-0", "300", "2024-03-20");
    let out = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(out.rent_received_eur.as_deref(), Some("700.00"));
    assert_eq!(out.outstanding_rent_eur.as_deref(), Some("300.00"));
    assert_eq!(out.noi_eur.as_deref(), Some("1000.00"));
    assert_eq!(out.dues[0].overdue_days, 26);
    assert!(out.dues[0].source_ids.contains(&"p1".into()));
    assert_eq!(out.cash_flow_after_financing_eur, None);
}

#[test]
fn future_payment_cannot_settle_old_date_and_late_history_survives_settlement() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    pay(&mut s, "p1", "600", "2024-03-05");
    pay(&mut s, "p2", "400", "2024-04-20");
    allocate(&mut s, "a1", "p1", "due-0", "600", "2024-03-05");
    allocate(&mut s, "a2", "p2", "due-0", "400", "2024-04-20");
    let march = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(march.rent_received_eur.as_deref(), Some("600.00"));
    assert_eq!(march.outstanding_rent_eur.as_deref(), Some("400.00"));
    let april = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-04-30")).unwrap();
    assert_eq!(april.outstanding_rent_eur.as_deref(), Some("0.00"));
    assert_eq!(april.dues[0].settled_on.as_deref(), Some("2024-04-20"));
    assert!(april.dues[0].historical_late);
}

#[test]
fn one_payment_can_cover_two_dues_without_duplicating_receipt() {
    let (r, p, mut s) = fixture();
    s.lease_terms[0].valid_to = Some("2024-02-01".into());
    s.lease_term_revisions = s.lease_terms.clone();
    let mut next = s.lease_terms[0].clone();
    next.id = "term2".into();
    next.revision_id = "term2-r".into();
    next.valid_from = "2024-02-01".into();
    next.valid_to = None;
    next.rent = "500".into();
    s.lease_terms.push(next.clone());
    s.lease_term_revisions.push(next);
    generate(&r, &p, &mut s, "2024-01-01", "2024-03-01");
    pay(&mut s, "p", "1500", "2024-02-05");
    allocate(&mut s, "a", "p", "due-0", "1000", "2024-02-05");
    allocate(&mut s, "b", "p", "due-1", "500", "2024-02-05");
    let out = rental_report(&r, &p, &s, &q("2024-01-01", "2024-03-01", "2024-02-29")).unwrap();
    assert_eq!(out.rent_received_eur.as_deref(), Some("1500.00"));
    assert_eq!(out.outstanding_rent_eur.as_deref(), Some("0.00"));
    s.allocations[1].amount = "501".into();
    assert!(validate_rentals(&r, &p, &s).is_err());
}

#[test]
fn annual_noi_and_yields_distinguish_operating_capital_and_financing() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2025-01-01", "2026-01-01");
    for (id, kind, amount) in [
        ("op", "operating", "2000"),
        ("works", "capital_works", "3000"),
        ("principal", "principal", "4000"),
    ] {
        s.expenses.push(RentalExpense {
            id: id.into(),
            revision_id: format!("{id}-r"),
            asset_id: "asset".into(),
            kind: kind.into(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: "2025-01-01".into(),
            paid_on: Some("2025-01-01".into()),
            recoverable: false,
            notes: String::new(),
        });
    }
    let out = rental_report(&r, &p, &s, &q("2025-01-01", "2026-01-01", "2025-12-31")).unwrap();
    assert_eq!(out.contractual_annual_rent_eur.as_deref(), Some("12000.00"));
    assert_eq!(out.noi_eur.as_deref(), Some("10000.00"));
    assert_eq!(out.gross_yield_percent.as_deref(), Some("6.00"));
    assert_eq!(out.net_yield_percent.as_deref(), Some("5.00"));
    assert_eq!(out.cash_flow_after_financing_eur, None);
    assert_eq!(out.occupation.occupied_unit_days, 365);
    assert_eq!(out.occupation.percent.as_deref(), Some("100.00"));
}

#[test]
fn leap_month_prorata_and_short_final_period_have_real_due_dates() {
    let (r, p, mut s) = fixture();
    s.leases[0].valid_from = "2024-02-15".into();
    s.leases[0].valid_to = Some("2024-03-01".into());
    s.lease_terms[0].valid_from = "2024-02-15".into();
    s.lease_terms[0].valid_to = Some("2024-03-01".into());
    s.lease_terms[0].due_day = 31;
    s.lease_term_revisions = s.lease_terms.clone();
    generate(&r, &p, &mut s, "2024-02-01", "2024-03-01");
    assert_eq!(s.dues[0].rent, "517.24");
    assert_eq!(s.dues[0].due_on, "2024-02-29");
    let (r, p, mut s) = fixture();
    s.leases[0].valid_to = Some("2024-01-03".into());
    s.lease_terms[0].valid_to = Some("2024-01-03".into());
    s.lease_term_revisions = s.lease_terms.clone();
    generate(&r, &p, &mut s, "2024-01-01", "2024-02-01");
    assert_eq!(s.dues[0].rent, "64.52");
    assert_eq!(s.dues[0].due_on, "2024-01-02");
}

#[test]
fn generation_key_is_independent_of_term_revision_and_request_clipping() {
    let (r, p, s) = fixture();
    let input = GenerateRentDuesInput {
        lease_id: "lease".into(),
        from_date: "2024-01-02".into(),
        to_date: "2024-02-01".into(),
        expected_sequence: 1,
        reason: "Test".into(),
    };
    let a = generate_rent_dues(&r, &p, &s, &input).unwrap();
    let mut corrected = s.clone();
    corrected.lease_terms[0].revision_id = "terms-corrected".into();
    corrected
        .lease_term_revisions
        .push(corrected.lease_terms[0].clone());
    let b = generate_rent_dues(&r, &p, &corrected, &input).unwrap();
    assert_eq!(a[0].generation_key, b[0].generation_key);
    assert_ne!(a[0].terms_revision_id, b[0].terms_revision_id);
    assert_eq!(a[0].period_start, "2024-01-01");
}

#[test]
fn contract_without_terms_or_generated_dues_is_unknown_not_vacancy() {
    let (r, p, mut s) = fixture();
    let report = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(report.economic_rent_eur, None);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "missing_rent_dues"));
    s.lease_terms.clear();
    let input = GenerateRentDuesInput {
        lease_id: "lease".into(),
        from_date: "2024-03-01".into(),
        to_date: "2024-04-01".into(),
        expected_sequence: 1,
        reason: "Test".into(),
    };
    assert!(generate_rent_dues(&r, &p, &s, &input).is_err());
    let out = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(out.contractual_annual_rent_eur, None);
}

#[test]
fn unexposed_lease_without_dues_or_terms_keeps_zero_and_its_explicit_cash_recipient() {
    let (mut r, p, mut s) = fixture();
    r.ownerships[0].owner_entity_id = "buyer".into();
    pay(&mut s, "advance", "100", "2024-03-05");
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.context = "family".into();
    query.asset_id = None;
    for without_terms in [false, true] {
        if without_terms {
            s.lease_terms.clear();
        }
        let report = rental_report(&r, &p, &s, &query).unwrap();
        assert_eq!(report.status, "complete");
        assert_eq!(report.contractual_annual_rent_eur.as_deref(), Some("0.00"));
        assert_eq!(report.economic_rent_eur.as_deref(), Some("0.00"));
        assert_eq!(report.rent_due_eur.as_deref(), Some("0.00"));
        assert_eq!(report.outstanding_rent_eur.as_deref(), Some("0.00"));
        assert_eq!(report.unallocated_payments_eur.as_deref(), Some("100.00"));
        assert!(!report
            .warnings
            .iter()
            .any(|w| matches!(w.code.as_str(), "missing_lease_terms" | "missing_rent_dues")));
    }
}

#[test]
fn unexposed_missing_dues_do_not_invalidate_another_propertys_known_income() {
    let (mut r, mut p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    r.assets.push(Asset {
        id: "outside-asset".into(),
        revision_id: "outside-asset-r".into(),
        ..r.assets[0].clone()
    });
    r.ownerships.push(Ownership {
        id: "outside-own".into(),
        revision_id: "outside-own-r".into(),
        asset_id: "outside-asset".into(),
        owner_entity_id: "buyer".into(),
        ..r.ownerships[0].clone()
    });
    p.properties.push(Property {
        id: "outside-property".into(),
        revision_id: "outside-property-r".into(),
        asset_id: "outside-asset".into(),
        ..p.properties[0].clone()
    });
    p.units.push(RentalUnit {
        id: "outside-unit".into(),
        revision_id: "outside-unit-r".into(),
        property_asset_id: "outside-asset".into(),
        ..p.units[0].clone()
    });
    s.leases.push(Lease {
        id: "outside-lease".into(),
        revision_id: "outside-lease-r".into(),
        unit_id: "outside-unit".into(),
        ..s.leases[0].clone()
    });
    let terms = LeaseTerms {
        id: "outside-terms".into(),
        revision_id: "outside-terms-r".into(),
        lease_id: "outside-lease".into(),
        ..s.lease_terms[0].clone()
    };
    s.lease_terms.push(terms.clone());
    s.lease_term_revisions.push(terms);
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.context = "family".into();
    query.asset_id = None;
    let report = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(report.status, "complete");
    assert_eq!(
        report.contractual_annual_rent_eur.as_deref(),
        Some("12000.00")
    );
    assert_eq!(report.economic_rent_eur.as_deref(), Some("1000.00"));
    assert_eq!(report.outstanding_rent_eur.as_deref(), Some("1000.00"));
    assert!(!report
        .warnings
        .iter()
        .any(|w| w.code == "missing_rent_dues"));
}

#[test]
fn missing_dues_only_during_zero_exposure_preserve_the_covered_accrual_segment() {
    let (mut r, p, mut s) = fixture();
    r.ownerships[0].owner_entity_id = "buyer".into();
    r.ownerships[0].valid_to = Some("2024-03-16".into());
    r.ownerships.push(Ownership {
        id: "family-own".into(),
        revision_id: "family-own-r".into(),
        owner_entity_id: "owner".into(),
        valid_from: "2024-03-16".into(),
        valid_to: None,
        ..r.ownerships[0].clone()
    });
    s.lease_terms[0].valid_from = "2024-03-16".into();
    s.lease_term_revisions = s.lease_terms.clone();
    generate(&r, &p, &mut s, "2024-03-16", "2024-04-01");
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.context = "family".into();
    let report = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(report.status, "complete");
    assert_eq!(report.economic_rent_eur.as_deref(), Some("516.13"));
    assert_eq!(report.outstanding_rent_eur.as_deref(), Some("516.13"));
    assert!(!report
        .warnings
        .iter()
        .any(|w| matches!(w.code.as_str(), "missing_lease_terms" | "missing_rent_dues")));
    // Unknown rights on the missing segment must still invalidate the totals.
    r.ownerships[0].economic_share = None;
    let unknown = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(unknown.status, "partial");
    assert_eq!(unknown.economic_rent_eur, None);
    assert_eq!(unknown.outstanding_rent_eur, None);
}

#[test]
fn deposit_is_cash_and_restitution_with_zero_rental_income() {
    let out = deposit_effect("2000").unwrap();
    assert_eq!(out.cash_increase, f("2000", "1"));
    assert_eq!(out.restitution_liability, f("2000", "1"));
    assert_eq!(out.rental_income, f("0", "1"));
    assert!(deposit_effect("-1").is_err());
}

#[test]
fn refunds_reverse_only_their_original_allocations_without_erasing_history() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    pay(&mut s, "p", "1000", "2024-03-05");
    allocate(&mut s, "a", "p", "due-0", "1000", "2024-03-05");
    let mut refund = s.payments[0].clone();
    refund.id = "refund".into();
    refund.revision_id = "refund-r".into();
    refund.kind = "refund".into();
    refund.amount = "-200".into();
    refund.effective_on = "2024-03-20".into();
    refund.original_payment_id = Some("p".into());
    s.payments.push(refund);
    allocate(&mut s, "undo", "refund", "due-0", "-200", "2024-03-20");
    let out = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(out.rent_received_eur.as_deref(), Some("800.00"));
    assert_eq!(out.outstanding_rent_eur.as_deref(), Some("200.00"));
    s.payments[1].amount = "-1001".into();
    assert!(validate_rentals(&r, &p, &s).is_err());
}

#[test]
fn invalid_currencies_dates_sources_overlaps_and_unexplained_payments_are_rejected() {
    let (r, p, s) = fixture();
    let mut bad = s.clone();
    bad.leases.push(bad.leases[0].clone());
    bad.leases[1].id = "other".into();
    bad.leases[1].revision_id = "other-r".into();
    assert!(validate_rentals(&r, &p, &bad).is_err());
    let mut bad = s.clone();
    pay(&mut bad, "p", "10", "2024-03-05");
    bad.payments[0].unlinked_reason.clear();
    assert!(validate_rentals(&r, &p, &bad).is_err());
    let mut bad = s.clone();
    generate(&r, &p, &mut bad, "2024-03-01", "2024-04-01");
    bad.dues[0].terms_revision_id = "missing".into();
    assert!(validate_rentals(&r, &p, &bad).is_err());
    let mut bad = s.clone();
    bad.knowledge_sequence = 2;
    assert!(validate_rentals(&r, &p, &bad).is_err());
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.knowledge_sequence = Some(9);
    assert!(rental_report(&r, &p, &s, &query).is_err());
}

#[test]
fn creditor_rights_are_split_during_accrual_and_survive_later_transfer() {
    let (mut r, p, mut s) = fixture();
    r.ownerships[0].valid_to = Some("2024-03-16".into());
    r.ownerships.push(Ownership {
        id: "buyer-own".into(),
        revision_id: "buyer-own-r".into(),
        owner_entity_id: "buyer".into(),
        valid_from: "2024-03-16".into(),
        valid_to: None,
        ..r.ownerships[0].clone()
    });
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.context = "person".into();
    query.person_id = Some("owner".into());
    let out = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(out.exact_totals["economic_rent"], f("15000", "31"));
    assert_eq!(out.economic_rent_eur.as_deref(), Some("483.87"));
    assert_eq!(out.outstanding_rent_eur.as_deref(), Some("483.87"));
    assert_eq!(out.rent_received_eur.as_deref(), Some("0.00"));
}

#[test]
fn missing_exchange_rate_does_not_turn_income_into_zero_and_value_is_never_cost() {
    let (mut r, p, mut s) = fixture();
    s.lease_terms[0].currency = "TND".into();
    s.lease_term_revisions = s.lease_terms.clone();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    r.valuations.clear();
    let out = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(out.economic_rent_eur, None);
    assert_eq!(out.gross_yield_percent, None);
    assert!(out
        .warnings
        .iter()
        .any(|w| w.code == "missing_exchange_rate"));
    assert!(out.warnings.iter().any(|w| w.code == "missing_valuation"));
}

#[test]
fn refund_of_allocated_cash_needs_same_date_atomic_reversal() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    pay(&mut s, "p", "500", "2024-03-05");
    allocate(&mut s, "a", "p", "due-0", "500", "2024-03-05");
    let mut refund = s.payments[0].clone();
    refund.id = "refund".into();
    refund.revision_id = "refund-r".into();
    refund.kind = "refund".into();
    refund.amount = "-200".into();
    refund.effective_on = "2024-03-20".into();
    refund.original_payment_id = Some("p".into());
    s.payments.push(refund);
    assert!(validate_rentals(&r, &p, &s).is_err());
    allocate(&mut s, "reverse", "refund", "due-0", "-200", "2024-03-20");
    assert!(validate_rentals(&r, &p, &s).is_ok());
    let out = rental_report(&r, &p, &s, &q("2024-03-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(out.rent_received_eur.as_deref(), Some("300.00"));
    assert_eq!(out.outstanding_rent_eur.as_deref(), Some("700.00"));
    s.allocations[1].effective_on = "2024-03-21".into();
    assert!(validate_rentals(&r, &p, &s).is_err());
}

#[test]
fn refund_of_unallocated_advance_reduces_future_allocation_capacity() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    pay(&mut s, "p", "1500", "2024-03-05");
    allocate(&mut s, "a", "p", "due-0", "1000", "2024-03-05");
    let mut refund = s.payments[0].clone();
    refund.id = "refund".into();
    refund.revision_id = "refund-r".into();
    refund.kind = "refund".into();
    refund.amount = "-200".into();
    refund.effective_on = "2024-03-20".into();
    refund.original_payment_id = Some("p".into());
    s.payments.push(refund);
    assert!(validate_rentals(&r, &p, &s).is_ok());
    s.dues[0].charges = "1000".into();
    let mut extra = s.allocations[0].clone();
    extra.id = "extra".into();
    extra.revision_id = "extra-r".into();
    extra.component = "charges".into();
    extra.amount = "400".into();
    extra.effective_on = "2024-03-21".into();
    s.allocations.push(extra);
    assert!(validate_rentals(&r, &p, &s).is_err());
}

#[test]
fn company_income_and_direct_receipts_never_imply_member_distribution() {
    let (mut r, p, mut s) = fixture();
    let mut company = person("company");
    company.kind = "company".into();
    r.entities.push(company);
    r.ownerships[0].owner_entity_id = "company".into();
    r.participations.push(Participation {
        id: "shares".into(),
        revision_id: "shares-r".into(),
        owner_entity_id: "owner".into(),
        company_id: "company".into(),
        legal_share: Some(f("1", "1")),
        economic_share: Some(f("1", "1")),
        voting_share: Some(f("1", "1")),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    });
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    pay(&mut s, "p", "1000", "2024-03-05");
    s.payments[0].recipient_entity_id = "company".into();
    allocate(&mut s, "a", "p", "due-0", "1000", "2024-03-05");
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.context = "person".into();
    query.person_id = Some("owner".into());
    let member = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(member.economic_rent_eur.as_deref(), Some("1000.00"));
    assert_eq!(member.rent_received_eur.as_deref(), Some("0.00"));
    query.context = "company".into();
    query.person_id = Some("company".into());
    let co = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(co.economic_rent_eur.as_deref(), Some("1000.00"));
    assert_eq!(co.rent_received_eur.as_deref(), Some("1000.00"));
}

#[test]
fn country_filter_uses_dated_profile_instead_of_current_country() {
    let (mut r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    let old = r.assets[0].clone();
    r.assets[0].country = "TN".into();
    r.assets[0].revision_id = "asset-new".into();
    r.asset_profile_revisions = vec![old, r.assets[0].clone()];
    r.profile_periods.push(ProfilePeriod {
        id: "old-country".into(),
        revision_id: "old-country-r".into(),
        target_kind: "asset".into(),
        target_id: "asset".into(),
        profile_revision_id: "asset-r".into(),
        valid_from: "2020-01-01".into(),
        valid_to: Some("2024-07-01".into()),
    });
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.country = Some("FR".into());
    assert_eq!(
        rental_report(&r, &p, &s, &query)
            .unwrap()
            .economic_rent_eur
            .as_deref(),
        Some("1000.00")
    );
    query.country = Some("TN".into());
    assert_eq!(
        rental_report(&r, &p, &s, &query)
            .unwrap()
            .economic_rent_eur
            .as_deref(),
        Some("0.00")
    );
    r.profile_periods.clear();
    query.country = Some("FR".into());
    let out = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(out.economic_rent_eur.as_deref(), Some("0.00"));
    assert!(out
        .warnings
        .iter()
        .any(|w| w.code == "undated_country_excluded"));
}

#[test]
fn income_and_outstanding_show_actual_fx_dates_and_rights_sources() {
    let (mut r, p, mut s) = fixture();
    s.lease_terms[0].currency = "TND".into();
    s.lease_term_revisions = s.lease_terms.clone();
    for (id, on, value) in [
        ("old-fx", "2024-01-01", "0.3"),
        ("new-fx", "2024-03-16", "0.4"),
    ] {
        r.exchange_rates.push(ExchangeRate {
            id: id.into(),
            revision_id: format!("{id}-r"),
            base_currency: "TND".into(),
            quote_currency: "EUR".into(),
            rate: value.into(),
            effective_on: on.into(),
            source: Some("Test".into()),
            notes: String::new(),
        });
    }
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.context = "family".into();
    let out = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(out.exact_totals["economic_rent"], f("10900", "31"));
    assert_eq!(out.rent_due_eur.as_deref(), Some("300.00"));
    assert_eq!(out.outstanding_rent_eur.as_deref(), Some("400.00"));
    for source in [
        "old-fx-r", "new-fx-r", "own-r", "member-r", "due-0-r", "terms-r",
    ] {
        assert!(
            out.source_ids.contains(&source.to_owned()),
            "missing source {source}"
        );
    }
    assert!(out
        .conversions
        .iter()
        .any(|c| c.date == "2024-03-05" && c.legs.iter().any(|l| l.revision_id == "old-fx-r")));
    assert!(out
        .conversions
        .iter()
        .any(|c| c.date == "2024-03-31" && c.legs.iter().any(|l| l.revision_id == "new-fx-r")));
    let keys: std::collections::BTreeSet<_> = out
        .conversions
        .iter()
        .map(|c| {
            (
                &c.date,
                &c.from_currency,
                &c.to_currency,
                c.legs
                    .iter()
                    .map(|l| l.revision_id.as_str())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    assert_eq!(keys.len(), out.conversions.len());
    assert!(out.conversions.iter().all(|c| c.amount_original == "1"));
}

#[test]
fn quarterly_and_yearly_schedules_keep_calendar_frequency() {
    let (r, p, mut s) = fixture();
    s.lease_terms[0].frequency = "quarterly".into();
    s.lease_terms[0].rent = "3000".into();
    s.lease_terms[0].due_day = 31;
    s.lease_term_revisions = s.lease_terms.clone();
    generate(&r, &p, &mut s, "2025-01-01", "2025-07-01");
    assert_eq!(s.dues.len(), 2);
    assert_eq!(s.dues[0].due_on, "2025-01-31");
    assert_eq!(s.dues[1].due_on, "2025-04-30");
    assert_eq!(s.dues[0].period_end, "2025-04-01");
    assert_eq!(s.dues[1].rent, "3000");
    let out = rental_report(&r, &p, &s, &q("2025-01-01", "2025-07-01", "2025-06-30")).unwrap();
    assert_eq!(out.contractual_annual_rent_eur.as_deref(), Some("12000.00"));
    s.dues.clear();
    s.lease_terms[0].frequency = "yearly".into();
    s.lease_terms[0].rent = "12000".into();
    s.lease_term_revisions = s.lease_terms.clone();
    generate(&r, &p, &mut s, "2025-01-01", "2026-01-01");
    assert_eq!(s.dues.len(), 1);
    assert_eq!(s.dues[0].period_end, "2026-01-01");
    assert_eq!(s.dues[0].rent, "12000");
}

#[test]
fn successive_leases_do_not_generate_rent_in_vacancy() {
    let (r, p, mut s) = fixture();
    s.leases[0].valid_to = Some("2024-02-01".into());
    s.lease_terms[0].valid_to = Some("2024-02-01".into());
    s.lease_term_revisions = s.lease_terms.clone();
    let mut next_lease = s.leases[0].clone();
    next_lease.id = "next-lease".into();
    next_lease.revision_id = "next-lease-r".into();
    next_lease.valid_from = "2024-03-01".into();
    next_lease.valid_to = Some("2024-04-01".into());
    s.leases.push(next_lease);
    let mut next_terms = s.lease_terms[0].clone();
    next_terms.id = "next-terms".into();
    next_terms.revision_id = "next-terms-r".into();
    next_terms.lease_id = "next-lease".into();
    next_terms.valid_from = "2024-03-01".into();
    next_terms.valid_to = Some("2024-04-01".into());
    s.lease_terms.push(next_terms.clone());
    s.lease_term_revisions.push(next_terms);
    generate(&r, &p, &mut s, "2024-01-01", "2024-04-01");
    let d = generate_rent_dues(
        &r,
        &p,
        &s,
        &GenerateRentDuesInput {
            lease_id: "next-lease".into(),
            from_date: "2024-01-01".into(),
            to_date: "2024-04-01".into(),
            expected_sequence: 1,
            reason: "Test".into(),
        },
    )
    .unwrap()
    .remove(0);
    let mut next = s.dues[0].clone();
    next.id = "next-due".into();
    next.revision_id = "next-due-r".into();
    next.lease_id = d.lease_id;
    next.terms_revision_id = d.terms_revision_id;
    next.period_start = d.period_start;
    next.period_end = d.period_end;
    next.due_on = d.due_on;
    next.generation_key = d.generation_key;
    s.dues.push(next);
    let out = rental_report(&r, &p, &s, &q("2024-01-01", "2024-04-01", "2024-03-31")).unwrap();
    assert_eq!(out.economic_rent_eur.as_deref(), Some("2000.00"));
    assert_eq!(out.occupation.available_unit_days, 91);
    assert_eq!(out.occupation.occupied_unit_days, 62);
    assert_eq!(out.occupation.percent.as_deref(), Some("68.13"));
    assert!(!out.warnings.iter().any(|w| w.code == "missing_rent_dues"));
}

#[test]
fn occupation_filters_unrelated_units_and_stops_when_rights_end() {
    let (mut r, mut p, mut s) = fixture();
    let mut asset = r.assets[0].clone();
    asset.id = "other-asset".into();
    asset.revision_id = "other-asset-r".into();
    r.assets.push(asset);
    let mut ownership = r.ownerships[0].clone();
    ownership.id = "other-own".into();
    ownership.revision_id = "other-own-r".into();
    ownership.asset_id = "other-asset".into();
    ownership.owner_entity_id = "buyer".into();
    r.ownerships.push(ownership);
    let mut property = p.properties[0].clone();
    property.id = "other-property".into();
    property.revision_id = "other-property-r".into();
    property.asset_id = "other-asset".into();
    p.properties.push(property);
    let mut unit = p.units[0].clone();
    unit.id = "other-unit".into();
    unit.revision_id = "other-unit-r".into();
    unit.property_asset_id = "other-asset".into();
    p.units.push(unit);
    s.leases[0].unit_id = "other-unit".into();
    generate(&r, &p, &mut s, "2024-03-01", "2024-04-01");
    let mut query = q("2024-03-01", "2024-04-01", "2024-03-31");
    query.asset_id = None;
    query.context = "person".into();
    query.person_id = Some("owner".into());
    let out = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(out.occupation.available_unit_days, 31);
    assert_eq!(out.occupation.occupied_unit_days, 0);
    assert_eq!(out.occupation.percent.as_deref(), Some("0.00"));
    r.ownerships[0].valid_to = Some("2024-03-16".into());
    let mut buyer = r.ownerships[0].clone();
    buyer.id = "buyer-own".into();
    buyer.revision_id = "buyer-own-r".into();
    buyer.owner_entity_id = "buyer".into();
    buyer.valid_from = "2024-03-16".into();
    buyer.valid_to = None;
    r.ownerships.push(buyer);
    let out = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(out.occupation.available_unit_days, 15);
    assert_eq!(out.occupation.percent.as_deref(), Some("0.00"));
    r.ownerships[0].economic_share = None;
    let out = rental_report(&r, &p, &s, &query).unwrap();
    assert_eq!(out.occupation.percent, None);
    assert!(out
        .warnings
        .iter()
        .any(|w| w.code == "occupation_scope_unknown"));
}

#[test]
fn annualized_net_yield_uses_elapsed_observation_not_future_request_end() {
    let (r, p, mut s) = fixture();
    generate(&r, &p, &mut s, "2025-01-01", "2026-01-01");
    let year = rental_report(&r, &p, &s, &q("2025-01-01", "2026-01-01", "2025-01-31")).unwrap();
    let january = rental_report(&r, &p, &s, &q("2025-01-01", "2025-02-01", "2025-01-31")).unwrap();
    assert_eq!(year.noi_eur.as_deref(), Some("1000.00"));
    assert_eq!(year.net_yield_percent.as_deref(), Some("5.89"));
    assert_eq!(year.net_yield_percent, january.net_yield_percent);
    assert!(year
        .warnings
        .iter()
        .any(|w| w.code == "observed_yield_period" && w.message.contains("31 jour")));
    let future = rental_report(&r, &p, &s, &q("2025-02-01", "2025-03-01", "2025-01-31")).unwrap();
    assert_eq!(future.net_yield_percent, None);
}
