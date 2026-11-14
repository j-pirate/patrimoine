use patrimoine_domain::*;

fn fraction(n: u64, d: u64) -> Fraction {
    Fraction {
        num: n.to_string(),
        den: d.to_string(),
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
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}
fn fixture() -> RegistrySnapshot {
    let mut s = RegistrySnapshot::default();
    for id in ["a", "b", "c", "outside"] {
        s.entities.push(entity(id, "person"));
    }
    for id in ["a", "b", "c"] {
        s.memberships.push(Membership {
            id: format!("member-{id}"),
            revision_id: format!("member-{id}-r"),
            person_id: id.into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            relationship: String::new(),
        });
    }
    s
}
fn company(s: &mut RegistrySnapshot, id: &str, mode: &str) {
    s.entities.push(entity(id, "company"));
    s.company_policies.push(CompanyPolicy {
        id: format!("policy-{id}"),
        revision_id: format!("policy-{id}-r"),
        company_id: id.into(),
        mode: mode.into(),
        coverage_complete: true,
        liability_qualified: true,
        valid_from: "2020-01-01".into(),
        valid_to: None,
    });
}
fn asset(s: &mut RegistrySnapshot, id: &str, amount: &str) {
    s.assets.push(Asset {
        id: id.into(),
        revision_id: format!("{id}-r"),
        name: id.into(),
        country: "TN".into(),
        category: "real_estate".into(),
        notes: String::new(),
        archived: false,
    });
    value(s, "asset", id, amount);
}
fn value(s: &mut RegistrySnapshot, kind: &str, target: &str, amount: &str) {
    s.valuations.push(Valuation {
        id: format!("value-{target}"),
        revision_id: format!("value-{target}-r"),
        target_kind: kind.into(),
        target_id: target.into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: "2020-01-01".into(),
        source: "Test synthétique".into(),
    });
}
fn own(s: &mut RegistrySnapshot, owner: &str, target: &str, n: u64, d: u64) {
    let id = format!("own-{owner}-{target}");
    s.ownerships.push(Ownership {
        id: id.clone(),
        revision_id: format!("{id}-r"),
        asset_id: target.into(),
        owner_entity_id: owner.into(),
        legal_share: Some(fraction(n, d)),
        economic_share: Some(fraction(n, d)),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    });
}
fn participation(s: &mut RegistrySnapshot, owner: &str, target: &str, n: u64, d: u64) {
    let id = format!("part-{owner}-{target}");
    s.participations.push(Participation {
        id: id.clone(),
        revision_id: format!("{id}-r"),
        owner_entity_id: owner.into(),
        company_id: target.into(),
        legal_share: Some(fraction(n, d)),
        economic_share: Some(fraction(n, d)),
        voting_share: None,
        valid_from: "2020-01-01".into(),
        valid_to: None,
    });
}
fn request(person: Option<&str>, date: &str) -> ConsolidationRequest {
    ConsolidationRequest {
        context: if person.is_some() { "person" } else { "family" }.into(),
        person_id: person.map(str::to_owned),
        date: date.into(),
        knowledge_sequence: None,
    }
}
fn report(s: &RegistrySnapshot, person: Option<&str>) -> ConsolidationReport {
    consolidate(s, &request(person, "2026-11-04")).unwrap()
}
fn warning(r: &ConsolidationReport, code: &str) -> bool {
    r.warnings.iter().any(|w| w.code == code)
}

#[test]
fn joint_family_asset_excludes_an_identified_outsider() {
    let mut s = fixture();
    asset(&mut s, "home", "400000");
    own(&mut s, "a", "home", 1, 2);
    own(&mut s, "b", "home", 3, 10);
    own(&mut s, "outside", "home", 1, 5);
    assert_eq!(report(&s, Some("a")).gross_eur, "200000.00");
    assert_eq!(report(&s, Some("b")).gross_eur, "120000.00");
    let family = report(&s, None);
    assert_eq!(family.gross_eur, "320000.00");
    assert_eq!(family.contributions.len(), 1);
    assert_eq!(family.contributions[0].weight, fraction(4, 5));
    assert_eq!(family.contributions[0].amount_original, "400000");
    assert!(family.gross_is_underlying_complete);
    assert!(warning(&family, "liabilities_not_inventoried"));
    assert_eq!(family.status, "partial");
}

#[test]
fn indirect_chain_multiplies_economic_rights_and_never_adds_parent_parts() {
    let mut s = fixture();
    company(&mut s, "x", "detailed");
    company(&mut s, "y", "detailed");
    participation(&mut s, "a", "x", 3, 5);
    participation(&mut s, "outside", "x", 2, 5);
    participation(&mut s, "x", "y", 1, 2);
    participation(&mut s, "outside", "y", 1, 2);
    asset(&mut s, "home", "400000");
    own(&mut s, "y", "home", 1, 1);
    value(&mut s, "company", "x", "900000");
    value(&mut s, "company", "y", "800000");
    let r = report(&s, Some("a"));
    assert_eq!(r.gross_eur, "120000.00");
    assert_eq!(r.contributions.len(), 1);
    assert_eq!(r.contributions[0].weight, fraction(3, 10));
    assert!(r.contributions[0].source_ids.contains(&"part-x-y".into()));
}

#[test]
fn multiple_paths_accumulate_without_a_visited_node_shortcut() {
    let mut s = fixture();
    company(&mut s, "x", "detailed");
    company(&mut s, "y", "detailed");
    participation(&mut s, "a", "x", 3, 5);
    participation(&mut s, "outside", "x", 2, 5);
    participation(&mut s, "a", "y", 1, 5);
    participation(&mut s, "x", "y", 1, 2);
    participation(&mut s, "outside", "y", 3, 10);
    asset(&mut s, "home", "100000");
    own(&mut s, "y", "home", 1, 1);
    let r = report(&s, Some("a"));
    assert_eq!(r.gross_eur, "50000.00");
    assert_eq!(r.contributions[0].weight, fraction(1, 2));
}

#[test]
fn opaque_boundaries_apply_to_paths_and_preserve_an_independent_direct_interest() {
    let mut s = fixture();
    company(&mut s, "x", "global_equity");
    company(&mut s, "y", "detailed");
    participation(&mut s, "a", "x", 3, 5);
    participation(&mut s, "outside", "x", 2, 5);
    participation(&mut s, "x", "y", 1, 2);
    participation(&mut s, "a", "y", 1, 5);
    participation(&mut s, "outside", "y", 3, 10);
    asset(&mut s, "home", "200000");
    own(&mut s, "y", "home", 1, 1);
    value(&mut s, "company", "x", "500000");
    let r = report(&s, Some("a"));
    assert_eq!(r.gross_eur, "340000.00");
    assert_eq!(r.global_equity_eur, "300000.00");
    assert!(!r.gross_is_underlying_complete);
    s.company_policies
        .iter_mut()
        .find(|p| p.company_id == "y")
        .unwrap()
        .mode = "global_equity".into();
    value(&mut s, "company", "y", "200000");
    let r = report(&s, Some("a"));
    assert_eq!(r.gross_eur, "340000.00");
    assert_eq!(r.global_equity_eur, "340000.00");
    assert!(r.contributions.iter().all(|c| c.kind == "global_equity"));
    s.valuations.retain(|v| v.target_kind != "asset");
    assert!(!warning(&report(&s, Some("a")), "missing_valuation"));
}

#[test]
fn unknown_is_not_zero_even_when_another_owner_is_reported_as_one_hundred_percent() {
    let mut s = fixture();
    asset(&mut s, "home", "400000");
    own(&mut s, "a", "home", 0, 1);
    own(&mut s, "outside", "home", 1, 1);
    s.ownerships[0].economic_share = None;
    let r = report(&s, Some("a"));
    assert!(warning(&r, "unknown_economic_rights"));
    assert_eq!(r.status, "non_calculable");
    s.ownerships[0].economic_share = Some(fraction(0, 1));
    let r = report(&s, Some("a"));
    assert!(!warning(&r, "unknown_economic_rights"));
    assert_eq!(r.gross_eur, "0.00");
}

#[test]
fn unassigned_remainder_is_not_assumed_to_be_outside_the_family() {
    let mut s = fixture();
    asset(&mut s, "home", "400000");
    own(&mut s, "a", "home", 1, 2);
    let r = report(&s, None);
    assert_eq!(r.gross_eur, "200000.00");
    assert!(warning(&r, "unknown_economic_rights"));
}

#[test]
fn foreign_and_future_values_are_never_replaced_by_an_invented_eur_amount() {
    let mut s = fixture();
    asset(&mut s, "home", "400000");
    own(&mut s, "a", "home", 1, 1);
    s.valuations[0].currency = "TND".into();
    let r = report(&s, None);
    assert_eq!(r.gross_eur, "0.00");
    assert!(r.contributions.is_empty());
    assert!(warning(&r, "missing_exchange_rate"));
    s.valuations[0].currency = "EUR".into();
    s.valuations[0].effective_on = "2027-01-01".into();
    let r = report(&s, None);
    assert!(r.contributions.is_empty());
    assert!(warning(&r, "missing_valuation"));
    assert_eq!(r.status, "non_calculable");
}

#[test]
fn membership_end_is_exclusive_and_archiving_a_profile_does_not_dispose_its_assets() {
    let mut s = fixture();
    asset(&mut s, "home", "100000");
    own(&mut s, "a", "home", 1, 1);
    s.entities[0].archived = true;
    assert_eq!(report(&s, None).gross_eur, "100000.00");
    s.memberships[0].valid_to = Some("2026-07-01".into());
    assert_eq!(
        consolidate(&s, &request(None, "2026-06-30"))
            .unwrap()
            .gross_eur,
        "100000.00"
    );
    assert_eq!(
        consolidate(&s, &request(None, "2026-07-01"))
            .unwrap()
            .gross_eur,
        "0.00"
    );
    assert_eq!(report(&s, Some("a")).gross_eur, "100000.00");
}

#[test]
fn three_exact_thirds_reconcile_before_final_rounding() {
    let mut s = fixture();
    asset(&mut s, "coin", "1");
    for owner in ["a", "b", "c"] {
        own(&mut s, owner, "coin", 1, 3);
    }
    assert_eq!(report(&s, None).gross_eur, "1.00");
    for person in ["a", "b", "c"] {
        let r = report(&s, Some(person));
        assert_eq!(r.gross_eur, "0.33");
        assert_eq!(r.exact_gross, fraction(1, 3));
    }
}

#[test]
fn structural_cycles_are_rejected_at_past_and_future_boundaries() {
    for start in ["2021-01-01", "2030-01-01"] {
        let mut s = fixture();
        company(&mut s, "x", "detailed");
        company(&mut s, "y", "detailed");
        participation(&mut s, "x", "y", 1, 2);
        participation(&mut s, "y", "x", 1, 2);
        s.participations[1].valid_from = start.into();
        assert!(validate_registry(&s).is_err());
    }
    let mut s = fixture();
    company(&mut s, "x", "detailed");
    company(&mut s, "y", "detailed");
    participation(&mut s, "x", "y", 1, 2);
    participation(&mut s, "y", "x", 1, 2);
    s.participations[0].valid_to = Some("2025-01-01".into());
    s.participations[1].valid_from = "2025-01-01".into();
    assert!(validate_registry(&s).is_ok());
}

#[test]
fn legal_economic_and_voting_rights_are_validated_independently() {
    let mut s = fixture();
    company(&mut s, "x", "detailed");
    participation(&mut s, "a", "x", 1, 2);
    participation(&mut s, "b", "x", 1, 2);
    s.participations[0].legal_share = Some(fraction(3, 5));
    s.participations[1].legal_share = Some(fraction(3, 5));
    assert!(validate_registry(&s).is_err());
    s.participations[0].legal_share = None;
    s.participations[1].legal_share = None;
    s.participations[0].voting_share = Some(fraction(3, 4));
    s.participations[1].voting_share = Some(fraction(1, 2));
    assert!(validate_registry(&s).is_err());
}

#[test]
fn overlapping_records_and_ambiguous_same_day_values_are_refused() {
    let mut s = fixture();
    asset(&mut s, "home", "100");
    own(&mut s, "a", "home", 1, 2);
    let mut second = s.ownerships[0].clone();
    second.id = "overlap".into();
    second.revision_id = "overlap-r".into();
    second.valid_from = "2022-01-01".into();
    s.ownerships.push(second);
    assert!(validate_registry(&s).is_err());
    s.ownerships.pop();
    let mut value = s.valuations[0].clone();
    value.id = "ambiguous".into();
    value.revision_id = "ambiguous-r".into();
    s.valuations.push(value);
    assert!(validate_registry(&s).is_err());
}

fn loan(creditor: &str, debtor: &str) -> LoanPosition {
    LoanPosition {
        id: "loan".into(),
        principal: "10000".into(),
        currency: "EUR".into(),
        creditors: vec![DebtParty {
            entity_id: creditor.into(),
            economic_share: Some(fraction(1, 1)),
        }],
        debtors: vec![DebtParty {
            entity_id: debtor.into(),
            economic_share: Some(fraction(1, 1)),
        }],
        valid_from: "2020-01-01".into(),
        valid_to: None,
        contingent: false,
    }
}

#[test]
fn reciprocal_family_loan_is_eliminated_without_removing_individual_positions() {
    let s = fixture();
    let loans = [loan("a", "b")];
    let family = consolidate_with_loans(&s, &request(None, "2026-01-01"), &loans, true).unwrap();
    assert_eq!(family.gross_eur, "0.00");
    assert_eq!(family.debt_eur, "0.00");
    assert_eq!(family.net_eur, "0.00");
    assert!(warning(&family, "internal_loan_elimination"));
    let a = consolidate_with_loans(&s, &request(Some("a"), "2026-01-01"), &loans, true).unwrap();
    let b = consolidate_with_loans(&s, &request(Some("b"), "2026-01-01"), &loans, true).unwrap();
    assert_eq!(a.net_eur, "10000.00");
    assert_eq!(b.net_eur, "-10000.00");
}

#[test]
fn partial_company_loan_eliminates_the_matching_fraction_and_keeps_the_net() {
    let mut s = fixture();
    company(&mut s, "x", "detailed");
    company(&mut s, "y", "detailed");
    participation(&mut s, "a", "x", 3, 4);
    participation(&mut s, "outside", "x", 1, 4);
    participation(&mut s, "b", "y", 2, 5);
    participation(&mut s, "outside", "y", 3, 5);
    let r =
        consolidate_with_loans(&s, &request(None, "2026-01-01"), &[loan("x", "y")], true).unwrap();
    assert_eq!(r.gross_eur, "3500.00");
    assert_eq!(r.debt_eur, "0.00");
    assert_eq!(r.net_eur, "3500.00");
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "internal_loan_elimination" && w.message.contains("4000.00")));
}

#[test]
fn personal_debt_is_not_reallocated_using_the_ownership_of_a_financed_asset() {
    let mut s = fixture();
    asset(&mut s, "home", "400000");
    own(&mut s, "a", "home", 1, 2);
    own(&mut s, "b", "home", 1, 2);
    let mut debt = loan("outside", "a");
    debt.principal = "150000".into();
    let r = consolidate_with_loans(&s, &request(Some("a"), "2026-01-01"), &[debt], true).unwrap();
    assert_eq!(r.gross_eur, "200000.00");
    assert_eq!(r.debt_eur, "150000.00");
    assert_eq!(r.net_eur, "50000.00");
}

#[test]
fn guarantee_is_not_an_existing_debt_and_opaque_loans_need_a_coverage_rule() {
    let mut s = fixture();
    let mut guarantee = loan("outside", "a");
    guarantee.contingent = true;
    let r = consolidate_with_loans(&s, &request(None, "2026-01-01"), &[guarantee], true).unwrap();
    assert_eq!(r.debt_eur, "0.00");
    assert!(warning(&r, "contingent_guarantee"));
    company(&mut s, "x", "global_equity");
    participation(&mut s, "a", "x", 1, 1);
    value(&mut s, "company", "x", "10000");
    let r =
        consolidate_with_loans(&s, &request(None, "2026-01-01"), &[loan("a", "x")], true).unwrap();
    assert_eq!(r.net_eur, "10000.00");
    assert!(warning(&r, "opaque_loan_coverage"));
    assert_eq!(r.status, "partial");
}

#[test]
fn unqualified_negative_equity_stays_analytical_not_a_personal_liability() {
    let mut s = fixture();
    company(&mut s, "x", "global_equity");
    participation(&mut s, "a", "x", 1, 1);
    value(&mut s, "company", "x", "-10000");
    s.company_policies[0].liability_qualified = false;
    let r = report(&s, None);
    assert!(warning(&r, "unqualified_negative_equity"));
    assert_eq!(r.debt_eur, "0.00");
    assert_ne!(r.status, "complete");
}

#[test]
fn date_and_context_validation_rejects_invalid_inputs() {
    let s = fixture();
    for date in [
        "2026-02-29",
        "2026-13-01",
        "2026-04-31",
        "0000-01-01",
        "2026-1-01",
    ] {
        assert!(consolidate(&s, &request(None, date)).is_err());
    }
    assert!(validate_date("2024-02-29").is_ok());
    assert!(consolidate(&s, &request(Some("unknown"), "2026-01-01")).is_err());
    let mut r = request(None, "2026-01-01");
    r.knowledge_sequence = Some(1);
    assert!(consolidate(&s, &r).is_err());
}

#[test]
fn french_form_normalization_does_not_weaken_the_persisted_decimal_contract() {
    assert_eq!(
        exact::normalize_decimal_input(" 1234,5600 ").unwrap(),
        "1234.56"
    );
    assert_eq!(exact::normalize_decimal_input("100.00").unwrap(), "100");
    assert!(exact::parse_decimal("1234,56").is_err());
    for invalid in ["1 2", "1,234.56", "1.2.3", "1..", "NaN", "1e5"] {
        assert!(exact::normalize_decimal_input(invalid).is_err());
    }
}
