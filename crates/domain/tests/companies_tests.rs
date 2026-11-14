use patrimoine_domain::*;

fn fraction(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
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
        activity: "Activité test".into(),
        notes: String::new(),
        archived: false,
    }
}
fn asset(id: &str) -> Asset {
    Asset {
        id: id.into(),
        revision_id: format!("{id}-r"),
        name: id.into(),
        country: "FR".into(),
        category: "bank".into(),
        notes: String::new(),
        archived: false,
    }
}
fn account(id: &str, owner: &str) -> TreasuryAccount {
    TreasuryAccount {
        id: id.into(),
        revision_id: format!("{id}-r"),
        asset_id: format!("{id}-asset"),
        kind: "bank".into(),
        currency: "EUR".into(),
        tracking_start: "2024-01-01".into(),
        legal_owner_entity_id: owner.into(),
        bank_name: "Banque test".into(),
        reference: String::new(),
        country: "FR".into(),
        storage_location: String::new(),
        notes: String::new(),
        archived: false,
    }
}
fn journal(id: &str, kind: &str, date: &str, rows: Vec<(&str, &str)>) -> TreasuryTransaction {
    TreasuryTransaction {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: kind.into(),
        effective_on: date.into(),
        label: id.into(),
        reference: String::new(),
        notes: String::new(),
        entries: rows
            .into_iter()
            .enumerate()
            .map(|(i, (account, amount))| TreasuryEntry {
                line_id: format!("line-{i}"),
                account_id: account.into(),
                amount: amount.into(),
                role: if kind == "opening" {
                    "opening"
                } else {
                    "principal"
                }
                .into(),
            })
            .collect(),
    }
}
fn period(id: &str, start: &str, end: &str, frequency: &str) -> CompanyFinancialPeriod {
    CompanyFinancialPeriod {
        id: id.into(),
        revision_id: format!("{id}-r"),
        company_id: "co".into(),
        period_start: start.into(),
        period_end: end.into(),
        frequency: frequency.into(),
        currency: "EUR".into(),
        revenue: Some("1000".into()),
        operating_expenses: Some("600".into()),
        operating_result: Some("400".into()),
        net_result: None,
        indicative_cash: Some("900000".into()),
        indicative_debt: Some("800000".into()),
        important_receivables: None,
        dividends_declared: None,
        indicative_global_value: Some("700000".into()),
        notes: String::new(),
    }
}
fn distribution(id: &str, kind: &str, amount: &str) -> CompanyDistribution {
    CompanyDistribution {
        id: id.into(),
        revision_id: format!("{id}-r"),
        company_id: "co".into(),
        beneficiary_entity_id: "a".into(),
        kind: kind.into(),
        effective_on: "2024-02-10".into(),
        amount: amount.into(),
        currency: "EUR".into(),
        related_declaration_id: None,
        unlinked_reason: if kind == "paid" {
            "Flux historique à rapprocher"
        } else {
            ""
        }
        .into(),
        notes: String::new(),
        cash_links: vec![],
    }
}

#[derive(Clone)]
struct Fixture {
    r: RegistrySnapshot,
    p: PropertySnapshot,
    u: RentalSnapshot,
    t: TreasurySnapshot,
    d: DebtSnapshot,
    c: CompanySnapshot,
}
impl Fixture {
    fn new() -> Self {
        let mut r = RegistrySnapshot {
            entities: vec![
                entity("co", "company"),
                entity("a", "person"),
                entity("outside", "person"),
            ],
            assets: vec![asset("co-bank-asset"), asset("a-bank-asset")],
            company_policies: vec![CompanyPolicy {
                id: "policy".into(),
                revision_id: "policy-r".into(),
                company_id: "co".into(),
                mode: "detailed".into(),
                coverage_complete: true,
                liability_qualified: true,
                valid_from: "2024-01-01".into(),
                valid_to: None,
            }],
            ..Default::default()
        };
        for (id, owner) in [("co-bank", "co"), ("a-bank", "a")] {
            r.ownerships.push(Ownership {
                id: format!("{id}-title"),
                revision_id: format!("{id}-title-r"),
                asset_id: format!("{id}-asset"),
                owner_entity_id: owner.into(),
                legal_share: Some(fraction("1", "1")),
                economic_share: Some(fraction("1", "1")),
                valid_from: "2024-01-01".into(),
                valid_to: None,
            });
        }
        Self {
            r,
            p: PropertySnapshot::default(),
            u: RentalSnapshot::default(),
            t: TreasurySnapshot {
                accounts: vec![account("co-bank", "co"), account("a-bank", "a")],
                transactions: vec![
                    journal(
                        "co-opening",
                        "opening",
                        "2024-01-01",
                        vec![("co-bank", "1000")],
                    ),
                    journal("a-opening", "opening", "2024-01-01", vec![("a-bank", "0")]),
                ],
                ..Default::default()
            },
            d: DebtSnapshot::default(),
            c: CompanySnapshot {
                financial_periods: vec![period("jan", "2024-01-01", "2024-02-01", "monthly")],
                ..Default::default()
            },
        }
    }
    fn valid(&self) -> Result<()> {
        validate_companies(&self.r, &self.p, &self.u, &self.t, &self.d, &self.c)
    }
    fn report(&self, date: &str, frequency: Option<&str>) -> CompanyReport {
        company_report(
            &self.r,
            &self.p,
            &self.u,
            &self.t,
            &self.d,
            &self.c,
            &CompanyReportRequest {
                company_id: "co".into(),
                date: date.into(),
                frequency: frequency.map(str::to_owned),
                knowledge_sequence: None,
            },
        )
        .unwrap()
    }
    fn paid(&mut self, amount: &str) {
        let minus = format!("-{amount}");
        self.t.transactions.push(journal(
            "dividend",
            "dividend",
            "2024-02-10",
            vec![("co-bank", &minus), ("a-bank", amount)],
        ));
        let mut d = distribution("paid", "paid", amount);
        d.unlinked_reason = String::new();
        d.cash_links = vec![
            CompanyDistributionCashLink {
                side: "payer".into(),
                transaction_revision_id: "dividend-r".into(),
                line_id: "line-0".into(),
                amount: minus,
            },
            CompanyDistributionCashLink {
                side: "beneficiary".into(),
                transaction_revision_id: "dividend-r".into(),
                line_id: "line-1".into(),
                amount: amount.into(),
            },
        ];
        self.c.distributions.push(d);
    }
    fn loan(&mut self) {
        self.d.contracts.push(DebtContract {
            id: "loan".into(),
            revision_id: "loan-r".into(),
            name: "Prêt test".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            notes: String::new(),
            archived: false,
        });
        self.d.terms.push(DebtTerms {
            id: "terms".into(),
            revision_id: "terms-r".into(),
            debt_id: "loan".into(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            principal_status: "complete".into(),
            interest_rate: None,
            interest_convention: "Non calculé".into(),
            parties: vec![
                FinancingParty {
                    line_id: "creditor".into(),
                    side: "creditor".into(),
                    entity_id: "outside".into(),
                    legal_share: Some(fraction("1", "1")),
                    economic_share: Some(fraction("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
                FinancingParty {
                    line_id: "co-debt".into(),
                    side: "debtor".into(),
                    entity_id: "co".into(),
                    legal_share: Some(fraction("1", "2")),
                    economic_share: Some(fraction("1", "2")),
                    liability_kind: "several".into(),
                    notes: String::new(),
                },
                FinancingParty {
                    line_id: "a-debt".into(),
                    side: "debtor".into(),
                    entity_id: "a".into(),
                    legal_share: Some(fraction("1", "2")),
                    economic_share: Some(fraction("1", "2")),
                    liability_kind: "several".into(),
                    notes: String::new(),
                },
            ],
            notes: String::new(),
        });
        self.d.movements.push(DebtMovement {
            id: "opening-loan".into(),
            revision_id: "opening-loan-r".into(),
            debt_id: "loan".into(),
            kind: "opening".into(),
            effective_on: "2024-01-01".into(),
            principal_change: "1000".into(),
            interest_paid: "0".into(),
            fees_paid: "0".into(),
            unlinked_reason: "Ouverture historique".into(),
            notes: String::new(),
        });
    }
}

#[test]
fn company_observations_are_not_banks_debts_or_valuations() {
    let s = Fixture::new();
    let r = s.report("2024-01-31", None);
    assert_eq!(
        r.accounts[0].report.balance_original.as_deref(),
        Some("1000.00")
    );
    assert!(r.debt_positions.is_empty());
    assert!(s.r.valuations.is_empty());
    assert_eq!(
        r.financial_periods[0].period.indicative_cash.as_deref(),
        Some("900000")
    );
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "company_indicators_not_positions"));
    assert_eq!(s.t.transactions.len(), 2);
}

#[test]
fn closed_periods_use_end_of_civil_observation_and_do_not_show_future_data() {
    let s = Fixture::new();
    assert!(s.report("2024-01-30", None).financial_periods.is_empty());
    assert_eq!(s.report("2024-01-31", None).financial_periods.len(), 1);
}

#[test]
fn frequencies_coexist_without_summing_and_can_be_filtered() {
    let mut s = Fixture::new();
    s.c.financial_periods.extend([
        period("q1", "2024-01-01", "2024-04-01", "quarterly"),
        period("annual", "2024-01-01", "2025-01-01", "annual"),
    ]);
    s.valid().unwrap();
    let all = s.report("2024-12-31", None);
    assert_eq!(all.financial_periods.len(), 3);
    assert!(all
        .warnings
        .iter()
        .any(|w| w.code == "mixed_company_frequencies"));
    let annual = s.report("2024-12-31", Some("annual"));
    assert_eq!(annual.financial_periods.len(), 1);
    assert_eq!(
        annual.financial_periods[0].period.revenue.as_deref(),
        Some("1000")
    );
}

#[test]
fn period_calendar_shape_and_overlaps_are_validated_independent_of_currency() {
    let s = Fixture::new();
    for (start, end, frequency) in [
        ("2024-01-02", "2024-02-02", "monthly"),
        ("2024-01-01", "2024-03-01", "monthly"),
        ("2024-01-01", "2024-02-01", "weekly"),
    ] {
        let mut invalid = s.clone();
        invalid.c.financial_periods = vec![period("bad", start, end, frequency)];
        assert!(invalid.valid().is_err());
    }
    let mut overlapping = s.clone();
    let mut duplicate = period("other", "2024-01-01", "2024-02-01", "monthly");
    duplicate.currency = "CHF".into();
    overlapping.c.financial_periods.push(duplicate);
    assert!(overlapping.valid().is_err());
    let mut fiscal = s;
    fiscal
        .c
        .financial_periods
        .push(period("fiscal", "2024-07-01", "2025-07-01", "annual"));
    fiscal.valid().unwrap();
}

#[test]
fn trends_require_adjacent_periods_same_frequency_and_currency() {
    let mut s = Fixture::new();
    let mut feb = period("feb", "2024-02-01", "2024-03-01", "monthly");
    feb.revenue = Some("1250".into());
    let mut april = period("apr", "2024-04-01", "2024-05-01", "monthly");
    april.currency = "CHF".into();
    s.c.financial_periods.extend([feb, april]);
    let r = s.report("2024-04-30", Some("monthly"));
    let feb = r
        .financial_periods
        .iter()
        .find(|p| p.period.id == "feb")
        .unwrap();
    let trend = feb.trends.iter().find(|t| t.metric == "revenue").unwrap();
    assert_eq!(trend.difference_original, "250.00");
    assert_eq!(trend.exact_percentage, Some(fraction("25", "1")));
    assert!(r
        .financial_periods
        .iter()
        .find(|p| p.period.id == "apr")
        .unwrap()
        .trends
        .is_empty());
}

#[test]
fn zero_and_unknown_bases_do_not_create_percentages_or_inferred_results() {
    let mut s = Fixture::new();
    s.c.financial_periods[0].revenue = Some("0".into());
    s.c.financial_periods[0].net_result = None;
    s.c.financial_periods
        .push(period("feb", "2024-02-01", "2024-03-01", "monthly"));
    let r = s.report("2024-02-29", None);
    let feb = r.financial_periods.last().unwrap();
    let revenue = feb.trends.iter().find(|t| t.metric == "revenue").unwrap();
    assert_eq!(revenue.difference_original, "1000.00");
    assert_eq!(revenue.percentage, None);
    assert_eq!(feb.period.net_result, None);
    assert!(!feb.trends.iter().any(|t| t.metric == "net_result"));
}

#[test]
fn discrepancy_is_exposed_without_replacing_declared_result() {
    let mut s = Fixture::new();
    s.c.financial_periods[0].operating_result = Some("350".into());
    let r = s.report("2024-01-31", None);
    let p = &r.financial_periods[0];
    assert_eq!(p.period.operating_result.as_deref(), Some("350"));
    assert_eq!(p.calculated_operating_result.as_deref(), Some("400.00"));
    assert_eq!(p.operating_result_difference.as_deref(), Some("-50.00"));
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "operating_result_discrepancy"));
}

#[test]
fn a_declaration_creates_no_cash_and_is_not_counted_as_a_payment() {
    let mut s = Fixture::new();
    s.c.distributions
        .push(distribution("declared", "declared", "500"));
    let r = s.report("2024-02-28", None);
    assert_eq!(s.t.transactions.len(), 2);
    assert_eq!(
        r.accounts[0].report.balance_original.as_deref(),
        Some("1000.00")
    );
    let d = &r.distributions[0];
    assert_eq!(d.paid_against_declaration.as_deref(), Some("0.00"));
    assert_eq!(d.remaining_declared.as_deref(), Some("500.00"));
    assert!(!d.payer_cash_complete && !d.beneficiary_cash_complete);
}

#[test]
fn actual_distribution_links_both_real_cash_faces_without_a_family_gain() {
    let mut s = Fixture::new();
    s.paid("1000");
    s.valid().unwrap();
    let r = s.report("2024-02-28", None);
    assert_eq!(
        r.accounts[0].report.balance_original.as_deref(),
        Some("0.00")
    );
    let personal = treasury_report(
        &s.r,
        &s.p,
        &s.u,
        &s.t,
        &TreasuryReportRequest {
            account_id: "a-bank".into(),
            date: "2024-02-28".into(),
            knowledge_sequence: None,
        },
    )
    .unwrap();
    assert_eq!(personal.balance_original.as_deref(), Some("1000.00"));
    assert!(r.distributions[0].payer_cash_complete && r.distributions[0].beneficiary_cash_complete);
    assert_eq!(s.t.transactions.len(), 3);
}

#[test]
fn historic_payment_without_cash_requires_a_reason_and_remains_partial() {
    let mut s = Fixture::new();
    s.c.distributions
        .push(distribution("history", "paid", "500"));
    s.valid().unwrap();
    let r = s.report("2024-02-28", None);
    assert_eq!(r.status, "partial");
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "unlinked_company_distribution"));
    assert_eq!(
        r.accounts[0].report.balance_original.as_deref(),
        Some("1000.00")
    );
    s.c.distributions[0].unlinked_reason = String::new();
    assert!(s.valid().is_err());
}

#[test]
fn dividend_links_require_exact_current_revision_holder_date_currency_and_sign() {
    let mut source = Fixture::new();
    source.paid("500");
    source.valid().unwrap();
    for change in 0..5 {
        let mut s = source.clone();
        match change {
            0 => s.c.distributions[0].cash_links[0].transaction_revision_id = "old-revision".into(),
            1 => s.t.accounts[0].legal_owner_entity_id = "outside".into(),
            2 => s.c.distributions[0].effective_on = "2024-02-11".into(),
            3 => s.c.distributions[0].currency = "CHF".into(),
            _ => s.c.distributions[0].cash_links[0].amount = "500".into(),
        }
        assert!(s.valid().is_err(), "mutation {change} should reject");
    }
}

#[test]
fn one_ledger_line_cannot_fund_multiple_dividends_above_its_capacity() {
    let mut s = Fixture::new();
    s.paid("1000");
    let mut second = s.c.distributions[0].clone();
    second.id = "second".into();
    second.revision_id = "second-r".into();
    s.c.distributions.push(second);
    assert!(s.valid().is_err());
}

#[test]
fn declaration_partial_payments_are_bounded_and_observed_at_their_dates() {
    let mut s = Fixture::new();
    let mut declaration = distribution("declared", "declared", "1000");
    declaration.effective_on = "2024-02-01".into();
    s.c.distributions.push(declaration);
    let mut paid = distribution("paid", "paid", "400");
    paid.related_declaration_id = Some("declared".into());
    s.c.distributions.push(paid);
    let early = s.report("2024-02-05", None);
    assert_eq!(
        early.distributions[0].remaining_declared.as_deref(),
        Some("1000.00")
    );
    let later = s.report("2024-02-28", None);
    assert_eq!(
        later
            .distributions
            .iter()
            .find(|p| p.distribution.id == "declared")
            .unwrap()
            .remaining_declared
            .as_deref(),
        Some("600.00")
    );
    s.c.distributions[1].amount = "1001".into();
    assert!(s.valid().is_err());
}

#[test]
fn company_loan_share_is_not_multiplied_a_second_time() {
    let mut s = Fixture::new();
    s.loan();
    let r = s.report("2024-02-28", None);
    let position = &r.debt_positions[0];
    assert_eq!(position.economic_share, Some(fraction("1", "2")));
    assert_eq!(position.amount_original.as_deref(), Some("500.00"));
    assert_eq!(position.exact_amount, Some(fraction("500", "1")));
}

#[test]
fn opaque_distribution_warns_to_review_equity_without_implicit_revaluation() {
    let mut s = Fixture::new();
    s.r.company_policies[0].mode = "global_equity".into();
    s.paid("500");
    let r = s.report("2024-02-28", None);
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "opaque_distribution_equity_review"));
    assert!(s.r.valuations.is_empty());
}

#[test]
fn foreign_snapshots_and_future_knowledge_are_refused_and_unknowns_survive() {
    let s = Fixture::new();
    assert!(company_report(
        &s.r,
        &s.p,
        &s.u,
        &s.t,
        &s.d,
        &s.c,
        &CompanyReportRequest {
            company_id: "co".into(),
            date: "2024-01-31".into(),
            frequency: None,
            knowledge_sequence: Some(1)
        }
    )
    .is_err());
    let mut different = s.clone();
    different.c.knowledge_sequence = 1;
    assert!(different.valid().is_err());
    let mut unknown = s;
    unknown.t.transactions.retain(|t| t.id != "co-opening");
    let r = unknown.report("2024-01-31", None);
    assert_eq!(r.accounts[0].report.balance_original, None);
    assert_eq!(r.financial_periods[0].period.net_result, None);
    assert_eq!(r.status, "partial");
}

#[test]
fn a_dividend_cannot_reuse_cash_already_allocated_to_a_loan() {
    let mut s = Fixture::new();
    s.loan();
    s.paid("500");
    s.d.movements.push(DebtMovement {
        id: "repayment".into(),
        revision_id: "repayment-r".into(),
        debt_id: "loan".into(),
        kind: "principal_repayment".into(),
        effective_on: "2024-02-10".into(),
        principal_change: "-100".into(),
        interest_paid: "0".into(),
        fees_paid: "0".into(),
        unlinked_reason: "Partiellement rapproché".into(),
        notes: String::new(),
    });
    s.t.cash_links.push(CashLink {
        id: "loan-link".into(),
        revision_id: "loan-link-r".into(),
        source_kind: "debt_movement".into(),
        source_id: "repayment".into(),
        source_revision_id: "repayment-r".into(),
        transaction_revision_id: "dividend-r".into(),
        line_id: "line-0".into(),
        amount: "-10".into(),
        component: "principal".into(),
        side: "debtor".into(),
        notes: String::new(),
    });
    validate_treasury(&s.r, &s.p, &s.u, &s.t).unwrap();
    validate_treasury_debt_links(&s.r, &s.t, &s.d).unwrap();
    assert!(s.valid().is_err());
}

#[test]
fn company_profile_policies_and_graph_respect_the_same_economic_date() {
    let mut s = Fixture::new();
    let mut old = s.r.entities[0].clone();
    old.revision_id = "co-old".into();
    old.display_name = "Ancienne société".into();
    s.r.entities[0].display_name = "Société corrigée".into();
    s.r.entities[0].residence_country = Some("TN".into());
    s.r.profile_revisions = vec![old, s.r.entities[0].clone()];
    s.r.profile_periods = vec![
        ProfilePeriod {
            id: "profile-before".into(),
            revision_id: "profile-before-r".into(),
            target_kind: "entity".into(),
            target_id: "co".into(),
            profile_revision_id: "co-old".into(),
            valid_from: "2024-01-01".into(),
            valid_to: Some("2024-07-01".into()),
        },
        ProfilePeriod {
            id: "profile-after".into(),
            revision_id: "profile-after-r".into(),
            target_kind: "entity".into(),
            target_id: "co".into(),
            profile_revision_id: "co-r".into(),
            valid_from: "2024-07-01".into(),
            valid_to: None,
        },
    ];
    s.r.company_policies[0].valid_to = Some("2024-07-01".into());
    s.r.company_policies.push(CompanyPolicy {
        id: "policy-global".into(),
        revision_id: "policy-global-r".into(),
        company_id: "co".into(),
        mode: "global_equity".into(),
        coverage_complete: true,
        liability_qualified: true,
        valid_from: "2024-07-01".into(),
        valid_to: None,
    });
    s.r.participations.push(Participation {
        id: "a-co".into(),
        revision_id: "a-co-r".into(),
        owner_entity_id: "a".into(),
        company_id: "co".into(),
        legal_share: Some(fraction("1", "2")),
        economic_share: Some(fraction("1", "2")),
        voting_share: Some(fraction("1", "2")),
        valid_from: "2024-01-01".into(),
        valid_to: Some("2024-07-01".into()),
    });
    let before = s.report("2024-06-30", None);
    assert_eq!(before.company.display_name, "Ancienne société");
    assert!(before.source_ids.iter().any(|id| id == "profile-before-r"));
    assert_eq!(before.company.residence_country.as_deref(), Some("FR"));
    assert_eq!(before.policy.unwrap().mode, "detailed");
    assert_eq!(before.owners.len(), 1);
    let after = s.report("2024-07-01", None);
    assert_eq!(after.company.display_name, "Société corrigée");
    assert!(after.source_ids.iter().any(|id| id == "profile-after-r"));
    assert_eq!(after.company.residence_country.as_deref(), Some("TN"));
    assert_eq!(after.policy.unwrap().mode, "global_equity");
    assert!(after.owners.is_empty());
}
