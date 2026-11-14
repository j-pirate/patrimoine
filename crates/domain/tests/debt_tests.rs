use patrimoine_domain::*;

fn fraction(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
    }
}
fn person(id: &str, kind: &str) -> Entity {
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
fn party(id: &str, entity: &str, side: &str, share: Option<Fraction>) -> FinancingParty {
    FinancingParty {
        line_id: id.into(),
        side: side.into(),
        entity_id: entity.into(),
        legal_share: share.clone(),
        economic_share: share,
        liability_kind: "single".into(),
        notes: String::new(),
    }
}
fn movement(
    id: &str,
    kind: &str,
    date: &str,
    principal: &str,
    interest: &str,
    fees: &str,
) -> DebtMovement {
    DebtMovement {
        id: id.into(),
        revision_id: format!("{id}-r"),
        debt_id: "loan".into(),
        kind: kind.into(),
        effective_on: date.into(),
        principal_change: principal.into(),
        interest_paid: interest.into(),
        fees_paid: fees.into(),
        unlinked_reason: "Source synthétique non rapprochée".into(),
        notes: "Justificatif synthétique".into(),
    }
}
fn fixture() -> (RegistrySnapshot, DebtSnapshot) {
    let r = RegistrySnapshot {
        entities: vec![
            person("a", "person"),
            person("b", "person"),
            person("outside", "person"),
            person("c", "company"),
        ],
        assets: vec![Asset {
            id: "house".into(),
            revision_id: "house-r".into(),
            name: "Bien synthétique".into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        }],
        ..Default::default()
    };
    let d = DebtSnapshot {
        contracts: vec![DebtContract {
            id: "loan".into(),
            revision_id: "loan-r".into(),
            name: "Prêt synthétique".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            notes: String::new(),
            archived: false,
        }],
        terms: vec![DebtTerms {
            id: "terms".into(),
            revision_id: "terms-r".into(),
            debt_id: "loan".into(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            principal_status: "complete".into(),
            interest_rate: Some("0.02".into()),
            interest_convention: "Taux informatif, aucun intérêt présumé".into(),
            parties: vec![
                party("creditor", "outside", "creditor", Some(fraction("1", "1"))),
                party("debtor", "a", "debtor", Some(fraction("1", "1"))),
            ],
            notes: String::new(),
        }],
        movements: vec![movement(
            "opening",
            "opening",
            "2024-01-01",
            "100000",
            "0",
            "0",
        )],
        ..Default::default()
    };
    (r, d)
}
fn report(r: &RegistrySnapshot, d: &DebtSnapshot, date: &str) -> DebtReport {
    debt_report(
        r,
        d,
        &DebtReportRequest {
            debt_id: "loan".into(),
            date: date.into(),
            knowledge_sequence: None,
        },
    )
    .unwrap()
}
fn adjustment(id: &str, kind: &str, date: &str, value: &str) -> DebtReceivableAdjustment {
    DebtReceivableAdjustment {
        id: id.into(),
        revision_id: format!("{id}-r"),
        debt_id: "loan".into(),
        creditor_entity_id: "outside".into(),
        kind: kind.into(),
        effective_on: date.into(),
        valuation_delta: value.into(),
        source: "Évaluation synthétique documentée".into(),
        notes: String::new(),
    }
}
fn schedule(id: &str, date: &str, principal: &str, interest: &str, fees: &str) -> DebtSchedule {
    DebtSchedule {
        id: id.into(),
        revision_id: format!("{id}-r"),
        debt_id: "loan".into(),
        due_on: date.into(),
        expected_principal: principal.into(),
        expected_interest: interest.into(),
        expected_fees: fees.into(),
        status: "planned".into(),
        notes: String::new(),
    }
}

#[test]
fn principal_interest_and_fees_stay_separate_and_asset_links_do_not_assign_borrowers() {
    let (r, mut d) = fixture();
    d.movements.push(movement(
        "paid",
        "principal_repayment",
        "2024-02-01",
        "-1000",
        "200",
        "10",
    ));
    d.asset_links.push(DebtAssetLink {
        id: "funding".into(),
        revision_id: "funding-r".into(),
        debt_id: "loan".into(),
        asset_id: "house".into(),
        relation_kind: "finances".into(),
        attributed_principal_share: Some(fraction("1", "2")),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: String::new(),
    });
    let report = report(&r, &d, "2024-02-01");
    assert_eq!(
        report.balance.principal_original.as_deref(),
        Some("99000.00")
    );
    assert_eq!(report.balance.interest_paid_original, "200.00");
    assert_eq!(report.balance.fees_paid_original, "10.00");
    assert_eq!(
        report
            .parties
            .iter()
            .find(|p| p.party.side == "debtor")
            .unwrap()
            .nominal_original
            .as_deref(),
        Some("99000.00")
    );
    assert_eq!(report.balance.exact_principal, Some(fraction("99000", "1")));
}

#[test]
fn forecasts_do_not_create_principal_or_payments_and_future_actuals_do_not_leak() {
    let (r, mut d) = fixture();
    d.schedules
        .push(schedule("forecast", "2024-12-01", "50000", "2000", "0"));
    d.movements.push(movement(
        "future",
        "principal_repayment",
        "2025-01-01",
        "-1000",
        "0",
        "0",
    ));
    let report = report(&r, &d, "2024-02-01");
    assert_eq!(
        report.balance.principal_original.as_deref(),
        Some("100000.00")
    );
    assert_eq!(report.schedules[0].paid_principal, "0.00");
    assert_eq!(report.schedules[0].overdue_days, 0);
    assert!(!report.source_ids.iter().any(|id| id == "future-r"));
}

#[test]
fn missing_opening_is_unknown_but_explicit_zero_is_known() {
    let (r, mut d) = fixture();
    d.movements.clear();
    let unknown = report(&r, &d, "2024-01-01");
    assert_eq!(unknown.status, "non_calculable");
    assert_eq!(unknown.balance.exact_principal, None);
    d.movements
        .push(movement("zero", "opening", "2024-01-01", "0", "0", "0"));
    let zero = report(&r, &d, "2024-01-01");
    assert_eq!(zero.balance.exact_principal, Some(fraction("0", "1")));
    assert_eq!(zero.principal_eur.as_deref(), Some("0.00"));
}

#[test]
fn missing_opening_does_not_turn_a_repayment_into_a_fabricated_negative_balance() {
    let (r, mut d) = fixture();
    d.movements = vec![movement(
        "partial",
        "principal_repayment",
        "2024-02-01",
        "-1000",
        "20",
        "0",
    )];
    assert!(validate_debts(&r, &d).is_ok());
    assert_eq!(
        report(&r, &d, "2024-03-01").balance.principal_original,
        None
    );
}

#[test]
fn negative_principal_is_rejected_at_earlier_dates_even_if_a_later_drawdown_cures_it() {
    let (r, mut d) = fixture();
    d.movements.push(movement(
        "excess",
        "principal_repayment",
        "2024-02-01",
        "-100001",
        "0",
        "0",
    ));
    d.movements
        .push(movement("later", "drawdown", "2024-03-01", "1", "0", "0"));
    assert!(validate_debts(&r, &d).is_err());
    d.terms[0].principal_status = "unknown".into();
    assert!(validate_debts(&r, &d).is_err());
}

#[test]
fn duplicate_or_misdated_openings_and_invalid_movement_signs_are_rejected() {
    let (r, d) = fixture();
    let mut duplicate = d.clone();
    duplicate
        .movements
        .push(movement("second", "opening", "2024-01-01", "1", "0", "0"));
    assert!(validate_debts(&r, &duplicate).is_err());
    let mut late = d.clone();
    late.movements[0].effective_on = "2024-01-02".into();
    assert!(validate_debts(&r, &late).is_err());
    for invalid in [
        movement("bad", "drawdown", "2024-02-01", "-1", "0", "0"),
        movement("bad", "principal_repayment", "2024-02-01", "1", "0", "0"),
        movement("bad", "write_off", "2024-02-01", "-1", "1", "0"),
    ] {
        let mut invalid_d = d.clone();
        invalid_d.movements.push(invalid);
        assert!(validate_debts(&r, &invalid_d).is_err());
    }
}

#[test]
fn incomplete_economic_parties_are_not_allocated_from_legal_rights_or_collateral() {
    let (r, mut d) = fixture();
    d.terms[0].parties[1].economic_share = None;
    let report = report(&r, &d, "2024-01-01");
    assert_eq!(report.status, "partial");
    let debtor = report
        .parties
        .iter()
        .find(|p| p.party.side == "debtor")
        .unwrap();
    assert_eq!(debtor.exact_nominal, None);
    assert_eq!(debtor.legal_maximum_original.as_deref(), Some("100000.00"));
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "unknown_debt_allocation"));
}

#[test]
fn joint_legal_liability_is_visible_without_duplicating_economic_principal() {
    let (r, mut d) = fixture();
    let mut a = party("a-joint", "a", "debtor", Some(fraction("1", "2")));
    a.legal_share = Some(fraction("1", "1"));
    a.liability_kind = "joint_and_several".into();
    let mut b = party("b-joint", "b", "debtor", Some(fraction("1", "2")));
    b.legal_share = Some(fraction("1", "1"));
    b.liability_kind = "joint_and_several".into();
    d.terms[0].parties.truncate(1);
    d.terms[0].parties.extend([a, b]);
    let report = report(&r, &d, "2024-01-01");
    let debtors: Vec<_> = report
        .parties
        .iter()
        .filter(|p| p.party.side == "debtor")
        .collect();
    assert!(debtors
        .iter()
        .all(|p| p.nominal_original.as_deref() == Some("50000.00")));
    assert!(debtors
        .iter()
        .all(|p| p.legal_maximum_original.as_deref() == Some("100000.00")));
}

#[test]
fn impairment_changes_creditor_value_but_never_debtor_nominal() {
    let (r, mut d) = fixture();
    d.receivable_adjustments
        .push(adjustment("loss", "impairment", "2024-02-01", "-20000"));
    let report = report(&r, &d, "2024-02-01");
    assert_eq!(
        report.balance.principal_original.as_deref(),
        Some("100000.00")
    );
    assert_eq!(
        report
            .parties
            .iter()
            .find(|p| p.party.side == "creditor")
            .unwrap()
            .carrying_value_original
            .as_deref(),
        Some("80000.00")
    );
    assert_eq!(
        report
            .parties
            .iter()
            .find(|p| p.party.side == "debtor")
            .unwrap()
            .carrying_value_original
            .as_deref(),
        Some("100000.00")
    );
}

#[test]
fn repayment_requires_traceable_impairment_reversal_and_reversal_cannot_exceed_prior_loss() {
    let (r, mut d) = fixture();
    d.receivable_adjustments
        .push(adjustment("loss", "impairment", "2024-02-01", "-20000"));
    d.movements.push(movement(
        "repaid",
        "principal_repayment",
        "2024-03-01",
        "-90000",
        "0",
        "0",
    ));
    assert!(validate_debts(&r, &d).is_err());
    d.receivable_adjustments
        .push(adjustment("reversal", "reversal", "2024-03-01", "10000"));
    assert!(validate_debts(&r, &d).is_ok());
    assert_eq!(
        report(&r, &d, "2024-03-01")
            .parties
            .iter()
            .find(|p| p.party.side == "creditor")
            .unwrap()
            .carrying_value_original
            .as_deref(),
        Some("0.00")
    );
    d.receivable_adjustments[1].valuation_delta = "20001".into();
    assert!(validate_debts(&r, &d).is_err());
}

#[test]
fn legal_writeoff_reduces_both_nominal_faces_without_creating_cash() {
    let (r, mut d) = fixture();
    d.movements.push(movement(
        "waiver",
        "write_off",
        "2024-02-01",
        "-100000",
        "0",
        "0",
    ));
    let report = report(&r, &d, "2024-02-01");
    assert_eq!(report.balance.principal_original.as_deref(), Some("0.00"));
    assert!(report
        .parties
        .iter()
        .all(|p| p.nominal_original.as_deref() == Some("0.00")));
    assert_eq!(report.balance.interest_paid_original, "0.00");
}

#[test]
fn guarantees_remain_contingent_and_do_not_add_to_principal() {
    let (r, mut d) = fixture();
    d.guarantees.push(DebtGuarantee {
        id: "guarantee".into(),
        revision_id: "guarantee-r".into(),
        debt_id: "loan".into(),
        guarantor_entity_id: "b".into(),
        limit_amount: Some("50000".into()),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: "Non appelée".into(),
    });
    let report = report(&r, &d, "2024-02-01");
    assert_eq!(
        report.balance.principal_original.as_deref(),
        Some("100000.00")
    );
    assert_eq!(report.guarantees.len(), 1);
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "contingent_guarantee"));
}

#[test]
fn payment_allocations_cover_multiple_schedules_without_multiplying_repayment() {
    let (r, mut d) = fixture();
    d.movements.push(movement(
        "paid",
        "principal_repayment",
        "2024-03-20",
        "-1000",
        "200",
        "10",
    ));
    d.schedules = vec![
        schedule("one", "2024-03-05", "600", "100", "5"),
        schedule("two", "2024-04-05", "400", "100", "5"),
    ];
    d.payment_allocations = vec![
        DebtPaymentAllocation {
            id: "allocation-a".into(),
            revision_id: "allocation-a-r".into(),
            movement_id: "paid".into(),
            schedule_id: "one".into(),
            effective_on: "2024-03-20".into(),
            principal: "600".into(),
            interest: "100".into(),
            fees: "5".into(),
        },
        DebtPaymentAllocation {
            id: "allocation-b".into(),
            revision_id: "allocation-b-r".into(),
            movement_id: "paid".into(),
            schedule_id: "two".into(),
            effective_on: "2024-03-20".into(),
            principal: "400".into(),
            interest: "100".into(),
            fees: "5".into(),
        },
    ];
    let report = report(&r, &d, "2024-04-30");
    assert_eq!(
        report.balance.principal_original.as_deref(),
        Some("99000.00")
    );
    assert!(report
        .schedules
        .iter()
        .all(|s| s.remaining_principal == "0.00"));
    assert!(report.schedules[0].historical_late);
    assert_eq!(
        report.schedules[0].settled_on.as_deref(),
        Some("2024-03-20")
    );
    d.payment_allocations[1].principal = "401".into();
    assert!(validate_debts(&r, &d).is_err());
}

#[test]
fn dated_terms_end_exclusively_and_overlap_or_party_aliases_are_rejected() {
    let (r, mut d) = fixture();
    d.terms[0].valid_to = Some("2024-07-01".into());
    let mut next = d.terms[0].clone();
    next.id = "new-terms".into();
    next.revision_id = "new-terms-r".into();
    next.valid_from = "2024-07-01".into();
    next.valid_to = None;
    next.parties[1].entity_id = "b".into();
    d.terms.push(next);
    assert_eq!(report(&r, &d, "2024-06-30").parties[1].party.entity_id, "a");
    assert_eq!(report(&r, &d, "2024-07-01").parties[1].party.entity_id, "b");
    d.terms[0].valid_to = Some("2024-07-02".into());
    assert!(validate_debts(&r, &d).is_err());
    d.terms[0].valid_to = Some("2024-07-01".into());
    d.terms[1].parties[1].entity_id = "outside".into();
    assert!(validate_debts(&r, &d).is_err());
}

#[test]
fn fx_is_applied_at_report_date_with_exact_sources_and_missing_rates_stay_unknown() {
    let (mut r, mut d) = fixture();
    d.contracts[0].currency = "TND".into();
    let missing = report(&r, &d, "2024-02-01");
    assert_eq!(missing.principal_eur, None);
    assert_eq!(
        missing.balance.principal_original.as_deref(),
        Some("100000.000")
    );
    r.exchange_rates = vec![ExchangeRate {
        id: "fx".into(),
        revision_id: "fx-r".into(),
        base_currency: "TND".into(),
        quote_currency: "EUR".into(),
        rate: "0.3".into(),
        effective_on: "2024-01-01".into(),
        source: Some("Cours synthétique".into()),
        notes: String::new(),
    }];
    let converted = report(&r, &d, "2024-02-01");
    assert_eq!(converted.principal_eur.as_deref(), Some("30000.00"));
    assert_eq!(converted.exact_principal_eur, Some(fraction("30000", "1")));
    assert!(converted.source_ids.iter().any(|id| id == "fx-r"));
    assert_eq!(
        report(&r, &d, "2023-12-31").balance.principal_original,
        None
    );
}

#[test]
fn historical_knowledge_and_invalid_civil_dates_are_guarded() {
    let (r, d) = fixture();
    assert!(debt_report(
        &r,
        &d,
        &DebtReportRequest {
            debt_id: "loan".into(),
            date: "2024-01-01".into(),
            knowledge_sequence: Some(1)
        }
    )
    .is_err());
    assert!(debt_report(
        &r,
        &d,
        &DebtReportRequest {
            debt_id: "loan".into(),
            date: "2023-02-29".into(),
            knowledge_sequence: None
        }
    )
    .is_err());
    let mut wrong = d.clone();
    wrong.knowledge_sequence = 1;
    assert!(validate_debts(&r, &wrong).is_err());
}

#[test]
fn opaque_coverage_is_dated_and_requires_source_without_changing_nominal() {
    let (r, mut d) = fixture();
    d.coverages.push(DebtCoverage {
        id: "coverage".into(),
        revision_id: "coverage-r".into(),
        debt_id: "loan".into(),
        company_id: "c".into(),
        side: "debtor".into(),
        valid_from: "2024-02-01".into(),
        valid_to: Some("2024-07-01".into()),
        included: true,
        source: "Valeur globale rapprochée au contrat".into(),
        notes: String::new(),
    });
    assert!(report(&r, &d, "2024-01-31").coverages.is_empty());
    assert_eq!(report(&r, &d, "2024-02-01").coverages.len(), 1);
    assert!(report(&r, &d, "2024-07-01").coverages.is_empty());
    assert_eq!(
        report(&r, &d, "2024-03-01")
            .balance
            .principal_original
            .as_deref(),
        Some("100000.00")
    );
    d.coverages[0].source.clear();
    assert!(validate_debts(&r, &d).is_err());
}

#[test]
fn financing_shares_and_collateral_do_not_silently_overallocate_principal() {
    let (r, mut d) = fixture();
    let link = DebtAssetLink {
        id: "link".into(),
        revision_id: "link-r".into(),
        debt_id: "loan".into(),
        asset_id: "house".into(),
        relation_kind: "collateral".into(),
        attributed_principal_share: Some(fraction("1", "1")),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: String::new(),
    };
    d.asset_links.push(link);
    assert!(validate_debts(&r, &d).is_err());
    d.asset_links[0].attributed_principal_share = None;
    assert!(validate_debts(&r, &d).is_ok());
}
