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
fn asset(id: &str) -> Asset {
    Asset {
        id: id.into(),
        revision_id: format!("{id}-r"),
        name: id.into(),
        country: "FR".into(),
        category: "other".into(),
        notes: String::new(),
        archived: false,
    }
}
fn title(id: &str, asset: &str, owner: &str) -> Ownership {
    Ownership {
        id: id.into(),
        revision_id: format!("{id}-r"),
        asset_id: asset.into(),
        owner_entity_id: owner.into(),
        legal_share: Some(f("1", "1")),
        economic_share: Some(f("1", "1")),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    }
}
fn transaction(id: &str, date: &str, amount: &str, kind: &str) -> TreasuryTransaction {
    TreasuryTransaction {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: kind.into(),
        effective_on: date.into(),
        label: id.into(),
        reference: String::new(),
        notes: String::new(),
        entries: vec![TreasuryEntry {
            line_id: format!("{id}-line"),
            account_id: "bank".into(),
            amount: amount.into(),
            role: if kind == "opening" {
                "opening"
            } else {
                "principal"
            }
            .into(),
        }],
    }
}
fn cash_link(
    id: &str,
    kind: &str,
    source: &str,
    tx: &str,
    amount: &str,
    component: &str,
    side: &str,
) -> CashLink {
    CashLink {
        id: id.into(),
        revision_id: format!("{id}-r"),
        source_kind: kind.into(),
        source_id: source.into(),
        source_revision_id: format!("{source}-r"),
        transaction_revision_id: format!("{tx}-r"),
        line_id: format!("{tx}-line"),
        amount: amount.into(),
        component: component.into(),
        side: side.into(),
        notes: String::new(),
    }
}
struct Fixture {
    r: RegistrySnapshot,
    p: PropertySnapshot,
    l: RentalSnapshot,
    t: TreasurySnapshot,
    d: DebtSnapshot,
    f: FinanceSnapshot,
}
fn fixture() -> Fixture {
    let r = RegistrySnapshot {
        entities: vec![person("a"), person("b"), person("tenant"), person("lender")],
        assets: vec![asset("house"), asset("bank-asset")],
        memberships: vec![
            Membership {
                id: "member-a".into(),
                revision_id: "member-a-r".into(),
                person_id: "a".into(),
                valid_from: "2020-01-01".into(),
                valid_to: None,
                relationship: String::new(),
            },
            Membership {
                id: "member-b".into(),
                revision_id: "member-b-r".into(),
                person_id: "b".into(),
                valid_from: "2020-01-01".into(),
                valid_to: None,
                relationship: String::new(),
            },
        ],
        ownerships: vec![
            title("house-title", "house", "a"),
            title("bank-title", "bank-asset", "a"),
        ],
        valuations: vec![Valuation {
            id: "estimate".into(),
            revision_id: "estimate-r".into(),
            target_kind: "asset".into(),
            target_id: "house".into(),
            amount: "200000".into(),
            currency: "EUR".into(),
            effective_on: "2020-01-01".into(),
            source: "Estimation synthétique".into(),
        }],
        ..Default::default()
    };
    let p = PropertySnapshot {
        properties: vec![Property {
            id: "property".into(),
            revision_id: "property-r".into(),
            asset_id: "house".into(),
            property_type: "house".into(),
            city: String::new(),
            address: String::new(),
            surface_m2: None,
            status: "rented".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            notes: String::new(),
        }],
        units: vec![RentalUnit {
            id: "unit".into(),
            revision_id: "unit-r".into(),
            property_asset_id: "house".into(),
            reference: "U1".into(),
            name: "Unité".into(),
            surface_m2: None,
            valid_from: "2020-01-01".into(),
            valid_to: None,
            available_for_rent: true,
            notes: String::new(),
        }],
        ..Default::default()
    };
    let mut l = RentalSnapshot {
        leases: vec![Lease {
            id: "lease".into(),
            revision_id: "lease-r".into(),
            unit_id: "unit".into(),
            tenant_entity_id: "tenant".into(),
            valid_from: "2023-01-01".into(),
            valid_to: Some("2024-01-01".into()),
            notes: String::new(),
        }],
        lease_terms: vec![LeaseTerms {
            id: "lease-terms".into(),
            revision_id: "lease-terms-r".into(),
            lease_id: "lease".into(),
            rent: "1000".into(),
            charges: "0".into(),
            currency: "EUR".into(),
            frequency: "monthly".into(),
            due_day: 5,
            deposit_required: "2000".into(),
            valid_from: "2023-01-01".into(),
            valid_to: Some("2024-01-01".into()),
            notes: String::new(),
        }],
        ..Default::default()
    };
    let proposals = generate_rent_dues(
        &r,
        &p,
        &l,
        &GenerateRentDuesInput {
            lease_id: "lease".into(),
            from_date: "2023-01-01".into(),
            to_date: "2024-01-01".into(),
            expected_sequence: 0,
            reason: "Année synthétique".into(),
        },
    )
    .unwrap();
    let mut t = TreasurySnapshot {
        accounts: vec![TreasuryAccount {
            id: "bank".into(),
            revision_id: "bank-r".into(),
            asset_id: "bank-asset".into(),
            kind: "bank".into(),
            currency: "EUR".into(),
            tracking_start: "2023-01-01".into(),
            legal_owner_entity_id: "a".into(),
            bank_name: "Banque synthétique".into(),
            reference: String::new(),
            country: "FR".into(),
            storage_location: String::new(),
            notes: String::new(),
            archived: false,
        }],
        transactions: vec![transaction("cash-opening", "2023-01-01", "0", "opening")],
        ..Default::default()
    };
    let mut claims = Vec::new();
    for (index, proposal) in proposals.into_iter().enumerate() {
        let due_id = format!("due-{index}");
        let payment_id = format!("rent-{index}");
        let tx_id = format!("receipt-{index}");
        l.dues.push(RentDue {
            id: due_id.clone(),
            revision_id: format!("{due_id}-r"),
            lease_id: proposal.lease_id,
            terms_revision_id: proposal.terms_revision_id,
            period_start: proposal.period_start.clone(),
            period_end: proposal.period_end,
            due_on: proposal.due_on.clone(),
            rent: proposal.rent,
            charges: proposal.charges,
            currency: proposal.currency,
            generation_key: proposal.generation_key,
            notes: proposal.notes,
        });
        l.payments.push(RentPayment {
            id: payment_id.clone(),
            revision_id: format!("{payment_id}-r"),
            lease_id: "lease".into(),
            kind: "receipt".into(),
            amount: "1000".into(),
            currency: "EUR".into(),
            effective_on: proposal.due_on.clone(),
            original_payment_id: None,
            recipient_entity_id: "a".into(),
            reference: String::new(),
            unlinked_reason: "Rapproché par le lien explicite".into(),
        });
        l.allocations.push(RentAllocation {
            id: format!("allocation-{index}"),
            revision_id: format!("allocation-{index}-r"),
            payment_id: payment_id.clone(),
            due_id: due_id.clone(),
            component: "rent".into(),
            amount: "1000".into(),
            currency: "EUR".into(),
            effective_on: proposal.due_on.clone(),
        });
        t.transactions
            .push(transaction(&tx_id, &proposal.due_on, "1000", "rent"));
        t.cash_links.push(cash_link(
            &format!("cash-rent-{index}"),
            "rent_payment",
            &payment_id,
            &tx_id,
            "1000",
            "full",
            "",
        ));
        claims.push(RentClaim {
            id: format!("claim-{index}"),
            revision_id: format!("claim-{index}-r"),
            due_id,
            parties: vec![RentClaimParty {
                entity_id: "a".into(),
                economic_share: Some(f("1", "1")),
            }],
            valid_from: proposal.period_start,
            valid_to: None,
            represented_by_debt_id: None,
            notes: String::new(),
        });
    }
    l.expenses.push(RentalExpense {
        id: "expense".into(),
        revision_id: "expense-r".into(),
        asset_id: "house".into(),
        kind: "operating".into(),
        amount: "2000".into(),
        currency: "EUR".into(),
        effective_on: "2023-06-15".into(),
        paid_on: Some("2023-06-15".into()),
        recoverable: false,
        notes: String::new(),
    });
    t.transactions.push(transaction(
        "paid-expense",
        "2023-06-15",
        "-2000",
        "payment",
    ));
    t.cash_links.push(cash_link(
        "cash-expense",
        "rental_expense",
        "expense",
        "paid-expense",
        "-2000",
        "full",
        "",
    ));
    let d = DebtSnapshot {
        contracts: vec![DebtContract {
            id: "debt".into(),
            revision_id: "debt-r".into(),
            name: "Financement synthétique".into(),
            currency: "EUR".into(),
            tracking_start: "2023-01-01".into(),
            notes: String::new(),
            archived: false,
        }],
        terms: vec![DebtTerms {
            id: "debt-terms".into(),
            revision_id: "debt-terms-r".into(),
            debt_id: "debt".into(),
            valid_from: "2023-01-01".into(),
            valid_to: None,
            principal_status: "complete".into(),
            interest_rate: None,
            interest_convention: String::new(),
            parties: vec![
                FinancingParty {
                    line_id: "creditor".into(),
                    side: "creditor".into(),
                    entity_id: "lender".into(),
                    legal_share: Some(f("1", "1")),
                    economic_share: Some(f("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
                FinancingParty {
                    line_id: "debtor".into(),
                    side: "debtor".into(),
                    entity_id: "a".into(),
                    legal_share: Some(f("1", "1")),
                    economic_share: Some(f("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
            ],
            notes: String::new(),
        }],
        movements: vec![
            DebtMovement {
                id: "debt-opening".into(),
                revision_id: "debt-opening-r".into(),
                debt_id: "debt".into(),
                kind: "opening".into(),
                effective_on: "2023-01-01".into(),
                principal_change: "100000".into(),
                interest_paid: "0".into(),
                fees_paid: "0".into(),
                unlinked_reason: String::new(),
                notes: String::new(),
            },
            DebtMovement {
                id: "repayment".into(),
                revision_id: "repayment-r".into(),
                debt_id: "debt".into(),
                kind: "principal_repayment".into(),
                effective_on: "2023-12-31".into(),
                principal_change: "-3500".into(),
                interest_paid: "400".into(),
                fees_paid: "100".into(),
                unlinked_reason: "Rapproché par les liens exacts".into(),
                notes: String::new(),
            },
        ],
        asset_links: vec![DebtAssetLink {
            id: "financing".into(),
            revision_id: "financing-r".into(),
            debt_id: "debt".into(),
            asset_id: "house".into(),
            relation_kind: "finances".into(),
            attributed_principal_share: Some(f("1", "1")),
            valid_from: "2023-01-01".into(),
            valid_to: None,
            notes: String::new(),
        }],
        ..Default::default()
    };
    t.transactions
        .push(transaction("paid-debt", "2023-12-31", "-4000", "loan"));
    for (component, amount) in [
        ("principal", "-3500"),
        ("interest", "-400"),
        ("fee", "-100"),
    ] {
        t.cash_links.push(cash_link(
            &format!("cash-debt-{component}"),
            "debt_movement",
            "repayment",
            "paid-debt",
            amount,
            component,
            "debtor",
        ));
    }
    let finance = FinanceSnapshot {
        coverages: vec![FinanceCoverage {
            id: "coverage".into(),
            revision_id: "coverage-r".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            assets_complete: true,
            debts_complete: true,
            rentals_complete: true,
            notes: "Périmètre synthétique exhaustivement renseigné".into(),
        }],
        rent_claims: claims,
        ..Default::default()
    };
    Fixture {
        r,
        p,
        l,
        t,
        d,
        f: finance,
    }
}
fn query(date: &str) -> RentalReportRequest {
    RentalReportRequest {
        context: "property".into(),
        person_id: None,
        asset_id: Some("house".into()),
        country: None,
        from_date: "2023-01-01".into(),
        to_date: "2024-01-01".into(),
        date: date.into(),
        knowledge_sequence: None,
    }
}
fn rental(v: &Fixture, q: &RentalReportRequest) -> RentalReport {
    financial_rental_report(&v.r, &v.p, &v.l, &v.t, &v.d, &v.f, q).unwrap()
}

#[test]
fn fully_linked_property_cashflow_matches_o04_without_counting_principal_as_operating_expense() {
    let v = fixture();
    let out = rental(&v, &query("2023-12-31"));
    assert_eq!(out.rent_received_eur.as_deref(), Some("12000.00"));
    assert_eq!(out.noi_eur.as_deref(), Some("10000.00"));
    assert_eq!(
        out.cash_flow_after_financing_eur.as_deref(),
        Some("6000.00")
    );
    assert_eq!(
        out.exact_totals["cash_flow_after_financing"],
        f("6000", "1")
    );
    assert!(out
        .source_ids
        .iter()
        .any(|id| id == "cash-debt-principal-r"));
}

#[test]
fn cashflow_stays_unknown_without_declared_coverage_or_exact_source_links() {
    let mut v = fixture();
    v.f.coverages[0].debts_complete = false;
    assert_eq!(
        rental(&v, &query("2023-12-31")).cash_flow_after_financing_eur,
        None
    );
    v.f.coverages[0].debts_complete = true;
    v.t.cash_links.retain(|l| l.source_id != "rent-0");
    let out = rental(&v, &query("2023-12-31"));
    assert_eq!(out.cash_flow_after_financing_eur, None);
    assert!(out
        .warnings
        .iter()
        .any(|w| w.code == "unlinked_cashflow_source"));
}

#[test]
fn analytical_property_debt_is_independent_of_the_borrowers_ownership_share() {
    let mut v = fixture();
    v.r.ownerships[0].legal_share = Some(f("1", "2"));
    v.r.ownerships[0].economic_share = Some(f("1", "2"));
    let mut other = title("outside-title", "house", "tenant");
    other.legal_share = Some(f("1", "2"));
    other.economic_share = Some(f("1", "2"));
    v.r.ownerships.push(other);
    let out = financial_property_report(
        &v.r,
        &v.p,
        &v.l,
        &v.t,
        &v.d,
        &v.f,
        &PropertyReportRequest {
            asset_id: "house".into(),
            date: "2023-06-01".into(),
            knowledge_sequence: None,
        },
    )
    .unwrap();
    assert_eq!(out.debt_remaining_eur.as_deref(), Some("100000.00"));
    assert_eq!(out.net_before_disposal_eur.as_deref(), Some("100000.00"));
    assert_eq!(out.exact_debt_remaining_eur, Some(f("100000", "1")));
}

#[test]
fn missing_analytic_financing_fraction_is_never_replaced_by_an_ownership_fraction() {
    let mut v = fixture();
    v.d.asset_links[0].attributed_principal_share = None;
    let property = financial_property_report(
        &v.r,
        &v.p,
        &v.l,
        &v.t,
        &v.d,
        &v.f,
        &PropertyReportRequest {
            asset_id: "house".into(),
            date: "2023-06-01".into(),
            knowledge_sequence: None,
        },
    )
    .unwrap();
    assert_eq!(property.debt_remaining_eur, None);
    assert_eq!(
        rental(&v, &query("2023-12-31")).cash_flow_after_financing_eur,
        None
    );
}

#[test]
fn applied_deposit_reduces_due_and_records_settlement_without_another_rent_receipt() {
    let mut v = fixture();
    v.l.dues.retain(|d| d.id == "due-2");
    v.l.payments.retain(|p| p.id == "rent-2");
    v.l.payments[0].amount = "400".into();
    v.l.allocations.retain(|a| a.payment_id == "rent-2");
    v.l.allocations[0].amount = "400".into();
    v.l.expenses.clear();
    v.f.rent_claims.retain(|c| c.due_id == "due-2");
    v.t.transactions
        .retain(|t| t.id == "cash-opening" || t.id == "receipt-2");
    v.t.transactions
        .iter_mut()
        .find(|t| t.id == "receipt-2")
        .unwrap()
        .entries[0]
        .amount = "400".into();
    v.t.cash_links.retain(|l| l.source_id == "rent-2");
    v.t.cash_links[0].amount = "400".into();
    v.t.deposits = vec![
        DepositMovement {
            id: "deposit".into(),
            revision_id: "deposit-r".into(),
            lease_id: "lease".into(),
            kind: "received".into(),
            amount: "2000".into(),
            currency: "EUR".into(),
            effective_on: "2023-03-10".into(),
            recipient_entity_id: "a".into(),
            due_id: None,
            component: String::new(),
            notes: "Dépôt reçu distinct d’un revenu".into(),
        },
        DepositMovement {
            id: "applied".into(),
            revision_id: "applied-r".into(),
            lease_id: "lease".into(),
            kind: "applied_to_due".into(),
            amount: "-300".into(),
            currency: "EUR".into(),
            effective_on: "2023-03-20".into(),
            recipient_entity_id: "a".into(),
            due_id: Some("due-2".into()),
            component: "rent".into(),
            notes: "Compensation justifiée".into(),
        },
    ];
    v.t.transactions
        .push(transaction("deposit-cash", "2023-03-10", "2000", "deposit"));
    v.t.cash_links.push(cash_link(
        "deposit-link",
        "deposit_movement",
        "deposit",
        "deposit-cash",
        "2000",
        "full",
        "",
    ));
    let mut q = query("2023-03-19");
    q.from_date = "2023-03-01".into();
    q.to_date = "2023-04-01".into();
    let before = rental(&v, &q);
    assert_eq!(before.dues[0].outstanding_rent, "600");
    q.date = "2023-03-31".into();
    let after = rental(&v, &q);
    assert_eq!(after.dues[0].outstanding_rent, "300");
    assert_eq!(after.rent_received_eur.as_deref(), Some("400.00"));
    assert_eq!(
        after.cash_flow_after_financing_eur.as_deref(),
        Some("400.00")
    );
    assert!(after.source_ids.iter().any(|id| id == "applied-r"));
    let mut final_settlement = v.t.deposits[1].clone();
    final_settlement.id = "applied-final".into();
    final_settlement.revision_id = "applied-final-r".into();
    final_settlement.effective_on = "2023-04-20".into();
    v.t.deposits.push(final_settlement);
    q.date = "2023-04-30".into();
    let settled = rental(&v, &q);
    assert_eq!(settled.dues[0].outstanding_rent, "0");
    assert_eq!(settled.dues[0].settled_on.as_deref(), Some("2023-04-20"));
    assert!(settled.dues[0].historical_late);
    assert_eq!(settled.rent_received_eur.as_deref(), Some("400.00"));
    assert_eq!(
        account_balance_at(&v.t, "bank", "2023-04-30")
            .unwrap()
            .unwrap()
            .to_string(),
        "2400"
    );
}

#[test]
fn family_cashflow_nets_visible_loan_faces_but_property_shows_its_debt_service() {
    let mut v = fixture();
    v.d.terms[0].parties[0].entity_id = "b".into();
    v.r.assets.push(asset("bank-b-asset"));
    v.r.ownerships
        .push(title("bank-b-title", "bank-b-asset", "b"));
    let mut account = v.t.accounts[0].clone();
    account.id = "bank-b".into();
    account.revision_id = "bank-b-r".into();
    account.asset_id = "bank-b-asset".into();
    account.legal_owner_entity_id = "b".into();
    v.t.accounts.push(account);
    let mut opening_b = transaction("opening-b", "2023-01-01", "0", "opening");
    opening_b.entries[0].account_id = "bank-b".into();
    v.t.transactions.push(opening_b);
    let paid =
        v.t.transactions
            .iter_mut()
            .find(|t| t.id == "paid-debt")
            .unwrap();
    paid.entries[0].amount = "-3900".into();
    paid.entries.push(TreasuryEntry {
        line_id: "creditor-cash-line".into(),
        account_id: "bank-b".into(),
        amount: "3900".into(),
        role: "principal".into(),
    });
    paid.entries.push(TreasuryEntry {
        line_id: "bank-fee-line".into(),
        account_id: "bank".into(),
        amount: "-100".into(),
        role: "fee".into(),
    });
    v.t.cash_links
        .iter_mut()
        .find(|l| l.source_kind == "debt_movement" && l.component == "fee")
        .unwrap()
        .line_id = "bank-fee-line".into();
    for (component, amount) in [("principal", "3500"), ("interest", "400")] {
        let mut link = cash_link(
            &format!("creditor-{component}"),
            "debt_movement",
            "repayment",
            "paid-debt",
            amount,
            component,
            "creditor",
        );
        link.line_id = "creditor-cash-line".into();
        v.t.cash_links.push(link);
    }
    assert_eq!(
        rental(&v, &query("2023-12-31"))
            .cash_flow_after_financing_eur
            .as_deref(),
        Some("6000.00")
    );
    let mut family = query("2023-12-31");
    family.context = "family".into();
    assert_eq!(
        rental(&v, &family).cash_flow_after_financing_eur.as_deref(),
        Some("9900.00")
    );
    v.t.cash_links.retain(|l| l.side != "creditor");
    assert_eq!(rental(&v, &family).cash_flow_after_financing_eur, None);
}

#[test]
fn property_financing_keeps_the_effective_fx_source_and_exact_euro_debt() {
    let mut v = fixture();
    v.d.contracts[0].currency = "TND".into();
    v.t.cash_links.retain(|l| l.source_kind != "debt_movement");
    v.r.exchange_rates.push(ExchangeRate {
        id: "financing-rate".into(),
        revision_id: "financing-rate-r".into(),
        base_currency: "TND".into(),
        quote_currency: "EUR".into(),
        rate: "0.3".into(),
        effective_on: "2023-01-01".into(),
        source: Some("Cotation synthétique".into()),
        notes: String::new(),
    });
    let out = financial_property_report(
        &v.r,
        &v.p,
        &v.l,
        &v.t,
        &v.d,
        &v.f,
        &PropertyReportRequest {
            asset_id: "house".into(),
            date: "2023-06-01".into(),
            knowledge_sequence: None,
        },
    )
    .unwrap();
    assert_eq!(out.debt_remaining_eur.as_deref(), Some("30000.00"));
    assert_eq!(out.exact_net_before_disposal_eur, Some(f("170000", "1")));
    assert!(out.source_ids.iter().any(|id| id == "financing-rate-r"));
    assert_eq!(
        out.financing_conversions[0].legs[0].effective_on,
        "2023-01-01"
    );
}

#[test]
fn family_cashflow_uses_membership_at_each_cash_date_instead_of_the_final_membership() {
    let mut v = fixture();
    v.r.memberships[0].valid_to = Some("2023-07-01".into());
    let mut q = query("2023-12-31");
    q.context = "family".into();
    assert_eq!(
        rental(&v, &q).cash_flow_after_financing_eur.as_deref(),
        Some("4000.00")
    );
    q.context = "person".into();
    q.person_id = Some("a".into());
    assert_eq!(
        rental(&v, &q).cash_flow_after_financing_eur.as_deref(),
        Some("6000.00")
    );
}
