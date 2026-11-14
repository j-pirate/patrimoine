use num_rational::BigRational;
use num_traits::Zero;
use patrimoine_domain::exact::fraction_of;
use patrimoine_domain::*;

fn f(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
    }
}
fn entity(id: &str) -> Entity {
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
fn account(id: &str, kind: &str, currency: &str) -> TreasuryAccount {
    TreasuryAccount {
        id: id.into(),
        revision_id: format!("{id}-r"),
        asset_id: format!("{id}-asset"),
        kind: kind.into(),
        currency: currency.into(),
        tracking_start: "2024-01-01".into(),
        legal_owner_entity_id: "a".into(),
        bank_name: if kind == "bank" {
            "Banque synthétique".into()
        } else {
            String::new()
        },
        reference: String::new(),
        country: "FR".into(),
        storage_location: if kind == "cash" {
            "Coffre synthétique".into()
        } else {
            String::new()
        },
        notes: String::new(),
        archived: false,
    }
}
fn tx(id: &str, kind: &str, date: &str, lines: &[(&str, &str, &str)]) -> TreasuryTransaction {
    TreasuryTransaction {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: kind.into(),
        effective_on: date.into(),
        label: id.into(),
        reference: String::new(),
        notes: String::new(),
        entries: lines
            .iter()
            .enumerate()
            .map(|(i, (account, amount, role))| TreasuryEntry {
                line_id: format!("{id}-line-{i}"),
                account_id: (*account).into(),
                amount: (*amount).into(),
                role: (*role).into(),
            })
            .collect(),
    }
}
fn fixture() -> (
    RegistrySnapshot,
    PropertySnapshot,
    RentalSnapshot,
    TreasurySnapshot,
) {
    let accounts = vec![
        account("eur", "bank", "EUR"),
        account("other", "bank", "EUR"),
        account("tnd", "cash", "TND"),
    ];
    let registry = RegistrySnapshot {
        entities: vec![entity("a"), entity("tenant")],
        assets: accounts.iter().map(|a| asset(&a.asset_id)).collect(),
        ..Default::default()
    };
    (
        registry,
        PropertySnapshot::default(),
        RentalSnapshot::default(),
        TreasurySnapshot {
            accounts,
            ..Default::default()
        },
    )
}
fn report(
    r: &RegistrySnapshot,
    p: &PropertySnapshot,
    l: &RentalSnapshot,
    s: &TreasurySnapshot,
    id: &str,
    date: &str,
) -> TreasuryReport {
    treasury_report(
        r,
        p,
        l,
        s,
        &TreasuryReportRequest {
            account_id: id.into(),
            date: date.into(),
            knowledge_sequence: None,
        },
    )
    .unwrap()
}

#[test]
fn opening_receipt_payment_and_internal_transfer_reconcile_exactly() {
    let (r, p, l, mut s) = fixture();
    s.transactions = vec![
        tx(
            "open",
            "opening",
            "2024-01-01",
            &[("eur", "100", "opening")],
        ),
        tx(
            "open-dest",
            "opening",
            "2024-01-01",
            &[("other", "0", "opening")],
        ),
        tx(
            "income",
            "receipt",
            "2024-01-02",
            &[("eur", "30", "principal")],
        ),
        tx(
            "expense",
            "payment",
            "2024-01-03",
            &[("eur", "-20", "principal")],
        ),
        tx(
            "transfer",
            "transfer",
            "2024-01-04",
            &[("eur", "-50", "principal"), ("other", "50", "principal")],
        ),
    ];
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-01-04")
            .balance_original
            .as_deref(),
        Some("60.00")
    );
    assert_eq!(
        report(&r, &p, &l, &s, "other", "2024-01-04")
            .balance_original
            .as_deref(),
        Some("50.00")
    );
    let total = account_balance_at(&s, "eur", "2024-01-04")
        .unwrap()
        .unwrap()
        + account_balance_at(&s, "other", "2024-01-04")
            .unwrap()
            .unwrap();
    assert_eq!(fraction_of(&total), f("110", "1"));
    assert_eq!(
        fraction_of(
            &account_balance_at(&s, "eur", "2024-01-03")
                .unwrap()
                .unwrap()
        ),
        f("110", "1")
    );
}

#[test]
fn executed_fx_uses_actual_principals_and_fees_once_not_a_manual_quote() {
    let (mut r, p, l, mut s) = fixture();
    r.exchange_rates.push(ExchangeRate {
        id: "rate".into(),
        revision_id: "rate-r".into(),
        base_currency: "TND".into(),
        quote_currency: "EUR".into(),
        rate: "0.3".into(),
        effective_on: "2024-01-01".into(),
        source: Some("Valorisation, pas cours exécuté".into()),
        notes: String::new(),
    });
    s.transactions = vec![
        tx(
            "open",
            "opening",
            "2024-01-01",
            &[("eur", "500", "opening")],
        ),
        tx(
            "open-tnd",
            "opening",
            "2024-01-01",
            &[("tnd", "0", "opening")],
        ),
        tx(
            "executed",
            "fx",
            "2024-01-02",
            &[
                ("eur", "-100", "principal"),
                ("tnd", "330", "principal"),
                ("eur", "-2", "fee"),
            ],
        ),
    ];
    let eur = report(&r, &p, &l, &s, "eur", "2024-01-02");
    assert_eq!(eur.balance_original.as_deref(), Some("398.00"));
    assert_eq!(eur.executed_fx[0].exact_rate, f("33", "10"));
    assert_eq!(eur.executed_fx[0].fees[0].amount, f("2", "1"));
    let tnd = report(&r, &p, &l, &s, "tnd", "2024-01-02");
    assert_eq!(tnd.balance_original.as_deref(), Some("330.000"));
    assert_eq!(tnd.balance_eur.as_deref(), Some("99.00"));
    assert!(tnd.source_ids.contains(&"rate-r".into()));
}

#[test]
fn missing_opening_is_unknown_and_explicit_zero_is_a_real_source() {
    let (r, p, l, mut s) = fixture();
    s.transactions.push(tx(
        "income",
        "receipt",
        "2024-01-02",
        &[("eur", "40", "principal")],
    ));
    let unknown = report(&r, &p, &l, &s, "eur", "2024-01-02");
    assert_eq!(unknown.balance_original, None);
    assert_eq!(unknown.known_movement_total, f("40", "1"));
    assert!(unknown
        .warnings
        .iter()
        .any(|w| w.code == "missing_account_opening"));
    s.transactions.push(tx(
        "open",
        "opening",
        "2024-01-01",
        &[("eur", "0", "opening")],
    ));
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-01-02")
            .balance_original
            .as_deref(),
        Some("40.00")
    );
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2023-12-31").balance_original,
        None
    );
}

#[test]
fn physical_cash_rejects_a_negative_historical_day_even_if_it_recovers_later() {
    let (r, p, l, mut s) = fixture();
    s.transactions = vec![
        tx(
            "open",
            "opening",
            "2024-01-01",
            &[("tnd", "100", "opening")],
        ),
        tx(
            "out",
            "payment",
            "2024-01-02",
            &[("tnd", "-150", "principal")],
        ),
        tx(
            "in",
            "receipt",
            "2024-01-03",
            &[("tnd", "100", "principal")],
        ),
    ];
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.transactions[2].effective_on = "2024-01-02".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_ok());
}

#[test]
fn bank_negative_balance_is_reported_as_overdraft_without_a_second_contract() {
    let (r, p, l, mut s) = fixture();
    s.transactions = vec![
        tx("open", "opening", "2024-01-01", &[("eur", "0", "opening")]),
        tx(
            "out",
            "payment",
            "2024-01-02",
            &[("eur", "-20", "principal")],
        ),
    ];
    let q = report(&r, &p, &l, &s, "eur", "2024-01-02");
    assert_eq!(q.exact_balance, Some(f("-20", "1")));
    assert!(q.overdraft);
    assert!(q.warnings.iter().any(|w| w.code == "bank_overdraft"));
}

#[test]
fn reconciliation_keeps_the_difference_and_never_changes_the_journal() {
    let (r, p, l, mut s) = fixture();
    s.transactions = vec![
        tx(
            "open",
            "opening",
            "2024-01-01",
            &[("eur", "100", "opening")],
        ),
        tx("in", "receipt", "2024-01-02", &[("eur", "10", "principal")]),
    ];
    s.reconciliations.push(AccountReconciliation {
        id: "statement".into(),
        revision_id: "statement-r".into(),
        account_id: "eur".into(),
        effective_on: "2024-01-02".into(),
        statement_balance: "100".into(),
        statement_reference: "Relevé synthétique".into(),
        document_id: None,
        notes: String::new(),
    });
    let q = report(&r, &p, &l, &s, "eur", "2024-01-02");
    assert_eq!(q.balance_original.as_deref(), Some("110.00"));
    assert_eq!(q.reconciliations[0].exact_difference, Some(f("-10", "1")));
    assert_eq!(s.transactions.len(), 2);
    s.transactions.remove(0);
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-01-02").reconciliations[0].difference,
        None
    );
}

#[test]
fn duplicate_openings_invalid_transfers_and_hidden_fees_are_rejected() {
    let (r, p, l, mut s) = fixture();
    s.transactions = vec![
        tx("a", "opening", "2024-01-01", &[("eur", "0", "opening")]),
        tx("b", "opening", "2024-01-01", &[("eur", "1", "opening")]),
    ];
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.transactions = vec![tx(
        "transfer",
        "transfer",
        "2024-01-02",
        &[("eur", "-50", "principal"), ("other", "49", "principal")],
    )];
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.transactions = vec![tx(
        "fx",
        "fx",
        "2024-01-02",
        &[("eur", "-50", "principal"), ("other", "50", "principal")],
    )];
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.transactions = vec![tx(
        "fx",
        "fx",
        "2024-01-02",
        &[
            ("eur", "-50", "principal"),
            ("tnd", "150", "principal"),
            ("eur", "2", "fee"),
        ],
    )];
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn future_transactions_and_unknown_knowledge_are_not_silently_applied() {
    let (r, p, l, mut s) = fixture();
    s.transactions = vec![
        tx("open", "opening", "2024-01-01", &[("eur", "10", "opening")]),
        tx(
            "future",
            "receipt",
            "2024-02-01",
            &[("eur", "90", "principal")],
        ),
    ];
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-01-31")
            .balance_original
            .as_deref(),
        Some("10.00")
    );
    assert!(treasury_report(
        &r,
        &p,
        &l,
        &s,
        &TreasuryReportRequest {
            account_id: "eur".into(),
            date: "2024-01-31".into(),
            knowledge_sequence: Some(1)
        }
    )
    .is_err());
    s.knowledge_sequence = 1;
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

fn rental_fixture() -> (
    RegistrySnapshot,
    PropertySnapshot,
    RentalSnapshot,
    TreasurySnapshot,
) {
    let (mut r, mut p, mut l, mut s) = fixture();
    r.assets.push(asset("home"));
    p.properties.push(Property {
        id: "property".into(),
        revision_id: "property-r".into(),
        asset_id: "home".into(),
        property_type: "house".into(),
        city: String::new(),
        address: String::new(),
        surface_m2: None,
        status: "rented".into(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: String::new(),
    });
    p.units.push(RentalUnit {
        id: "unit".into(),
        revision_id: "unit-r".into(),
        property_asset_id: "home".into(),
        reference: "UNIT".into(),
        name: "Unité".into(),
        surface_m2: None,
        valid_from: "2024-01-01".into(),
        valid_to: None,
        available_for_rent: true,
        notes: String::new(),
    });
    l.leases.push(Lease {
        id: "lease".into(),
        revision_id: "lease-r".into(),
        unit_id: "unit".into(),
        tenant_entity_id: "tenant".into(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        notes: String::new(),
    });
    let term = LeaseTerms {
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
    l.lease_terms.push(term.clone());
    l.lease_term_revisions.push(term);
    l.dues.push(RentDue {
        id: "due".into(),
        revision_id: "due-r".into(),
        lease_id: "lease".into(),
        terms_revision_id: "terms-r".into(),
        period_start: "2024-03-01".into(),
        period_end: "2024-04-01".into(),
        due_on: "2024-03-05".into(),
        rent: "1000".into(),
        charges: "0".into(),
        currency: "EUR".into(),
        generation_key: "key".into(),
        notes: String::new(),
    });
    s.transactions.push(tx(
        "open",
        "opening",
        "2024-01-01",
        &[("eur", "0", "opening")],
    ));
    (r, p, l, s)
}
fn payment(id: &str, amount: &str) -> RentPayment {
    RentPayment {
        id: id.into(),
        revision_id: format!("{id}-r"),
        lease_id: "lease".into(),
        kind: "receipt".into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: "2024-03-05".into(),
        original_payment_id: None,
        recipient_entity_id: "a".into(),
        reference: String::new(),
        unlinked_reason: "Paiement synthétique".into(),
    }
}
fn link(
    id: &str,
    source: &str,
    source_rev: &str,
    transaction: &str,
    line: usize,
    amount: &str,
) -> CashLink {
    CashLink {
        id: id.into(),
        revision_id: format!("{id}-r"),
        source_kind: "rent_payment".into(),
        source_id: source.into(),
        source_revision_id: source_rev.into(),
        transaction_revision_id: format!("{transaction}-r"),
        line_id: format!("{transaction}-line-{line}"),
        amount: amount.into(),
        component: "full".into(),
        side: String::new(),
        notes: String::new(),
    }
}
fn deposit(id: &str, kind: &str, amount: &str, date: &str) -> DepositMovement {
    DepositMovement {
        id: id.into(),
        revision_id: format!("{id}-r"),
        lease_id: "lease".into(),
        kind: kind.into(),
        amount: amount.into(),
        currency: "EUR".into(),
        effective_on: date.into(),
        recipient_entity_id: "a".into(),
        due_id: if kind == "applied_to_due" {
            Some("due".into())
        } else {
            None
        },
        component: if kind == "applied_to_due" {
            "rent".into()
        } else {
            String::new()
        },
        notes: "Décision synthétique justifiée".into(),
    }
}

#[test]
fn one_bank_credit_can_match_two_sources_but_capacities_never_duplicate_cash() {
    let (r, p, mut l, mut s) = rental_fixture();
    l.payments = vec![payment("p1", "400"), payment("p2", "200")];
    s.transactions.push(tx(
        "receipt",
        "rent",
        "2024-03-05",
        &[("eur", "600", "principal")],
    ));
    s.cash_links = vec![
        link("l1", "p1", "p1-r", "receipt", 0, "400"),
        link("l2", "p2", "p2-r", "receipt", 0, "200"),
    ];
    assert!(validate_treasury(&r, &p, &l, &s).is_ok());
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-03-05")
            .balance_original
            .as_deref(),
        Some("600.00")
    );
    l.payments[1].amount = "300".into();
    s.cash_links[1].amount = "300".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn linked_source_and_cash_revisions_dates_currency_and_beneficiary_are_exact() {
    let (r, p, mut l, mut s) = rental_fixture();
    l.payments.push(payment("p", "400"));
    s.transactions.push(tx(
        "receipt",
        "rent",
        "2024-03-05",
        &[("eur", "400", "principal")],
    ));
    s.cash_links
        .push(link("link", "p", "p-r", "receipt", 0, "400"));
    assert!(validate_treasury(&r, &p, &l, &s).is_ok());
    s.cash_links[0].source_revision_id = "old-r".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.cash_links[0].source_revision_id = "p-r".into();
    s.transactions[1].revision_id = "new-r".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.transactions[1].revision_id = "receipt-r".into();
    s.transactions[1].effective_on = "2024-03-06".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.transactions[1].effective_on = "2024-03-05".into();
    s.accounts[0].legal_owner_entity_id = "tenant".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn confirmed_opening_cannot_be_reclassified_as_a_rental_cash_flow() {
    let (r, p, mut l, mut s) = rental_fixture();
    let mut pay = payment("p", "400");
    pay.effective_on = "2024-01-01".into();
    l.payments.push(pay);
    s.transactions[0].entries[0].amount = "400".into();
    s.cash_links
        .push(link("link", "p", "p-r", "open", 0, "400"));
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn received_deposit_creates_cash_and_obligation_but_no_rent_payment() {
    let (r, p, l, mut s) = rental_fixture();
    s.deposits
        .push(deposit("dep", "received", "2000", "2024-01-02"));
    s.transactions.push(tx(
        "dep-cash",
        "deposit",
        "2024-01-02",
        &[("eur", "2000", "principal")],
    ));
    let mut matched = link("matched", "dep", "dep-r", "dep-cash", 0, "2000");
    matched.source_kind = "deposit_movement".into();
    s.cash_links.push(matched);
    validate_treasury(&r, &p, &l, &s).unwrap();
    assert_eq!(
        fraction_of(&deposit_obligation_at(&s, "lease", "a", "2024-01-02").unwrap()),
        f("2000", "1")
    );
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-01-02")
            .balance_original
            .as_deref(),
        Some("2000.00")
    );
    assert!(l.payments.is_empty());
    assert_eq!(deposit_effect("2000").unwrap().rental_income, f("0", "1"));
}

#[test]
fn applying_deposit_settles_due_and_reduces_obligation_without_new_bank_cash() {
    let (r, p, l, mut s) = rental_fixture();
    s.deposits = vec![
        deposit("dep", "received", "2000", "2024-01-02"),
        deposit("apply", "applied_to_due", "-300", "2024-03-05"),
    ];
    s.transactions.push(tx(
        "dep-cash",
        "deposit",
        "2024-01-02",
        &[("eur", "2000", "principal")],
    ));
    validate_treasury(&r, &p, &l, &s).unwrap();
    assert_eq!(
        fraction_of(&deposit_applied_at(&s, "due", "rent", "2024-03-05").unwrap()),
        f("300", "1")
    );
    assert_eq!(
        fraction_of(&deposit_obligation_at(&s, "lease", "a", "2024-03-05").unwrap()),
        f("1700", "1")
    );
    assert_eq!(
        fraction_of(
            &account_balance_at(&s, "eur", "2024-03-05")
                .unwrap()
                .unwrap()
        ),
        f("2000", "1")
    );
    assert_eq!(s.transactions.len(), 2);
    let mut wrong = link("wrong", "apply", "apply-r", "dep-cash", 0, "300");
    wrong.source_kind = "deposit_movement".into();
    s.cash_links.push(wrong);
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn deposit_return_is_a_cash_outflow_and_historical_capacity_is_not_clamped() {
    let (r, p, l, mut s) = rental_fixture();
    s.deposits = vec![
        deposit("dep", "received", "2000", "2024-01-02"),
        deposit("apply", "applied_to_due", "-300", "2024-03-05"),
        deposit("returned", "returned", "-500", "2024-03-20"),
    ];
    s.transactions.extend([
        tx(
            "dep-cash",
            "deposit",
            "2024-01-02",
            &[("eur", "2000", "principal")],
        ),
        tx(
            "return-cash",
            "deposit",
            "2024-03-20",
            &[("eur", "-500", "principal")],
        ),
    ]);
    let mut linked = link(
        "return-link",
        "returned",
        "returned-r",
        "return-cash",
        0,
        "-500",
    );
    linked.source_kind = "deposit_movement".into();
    s.cash_links.push(linked);
    validate_treasury(&r, &p, &l, &s).unwrap();
    assert_eq!(
        fraction_of(&deposit_obligation_at(&s, "lease", "a", "2024-03-20").unwrap()),
        f("1200", "1")
    );
    assert_eq!(
        fraction_of(
            &account_balance_at(&s, "eur", "2024-03-20")
                .unwrap()
                .unwrap()
        ),
        f("1500", "1")
    );
    s.deposits[2].amount = "-1800".into();
    s.cash_links.clear();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn deposit_and_payment_combined_cannot_overpay_one_due_or_precede_exigibility() {
    let (r, p, mut l, mut s) = rental_fixture();
    s.deposits = vec![
        deposit("dep", "received", "2000", "2024-01-02"),
        deposit("apply", "applied_to_due", "-300", "2024-03-05"),
    ];
    l.payments.push(payment("p", "800"));
    l.allocations.push(RentAllocation {
        id: "a1".into(),
        revision_id: "a1-r".into(),
        payment_id: "p".into(),
        due_id: "due".into(),
        component: "rent".into(),
        amount: "800".into(),
        currency: "EUR".into(),
        effective_on: "2024-03-05".into(),
    });
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    l.allocations[0].amount = "700".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_ok());
    s.deposits[1].effective_on = "2024-03-04".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn account_specialization_cannot_duplicate_a_property_or_another_account() {
    let (r, p, l, mut s) = rental_fixture();
    s.accounts[0].asset_id = "home".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    s.accounts[0].asset_id = s.accounts[1].asset_id.clone();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn source_and_line_capacity_are_exact_for_fractional_money() {
    let (r, p, mut l, mut s) = rental_fixture();
    l.payments.push(payment("p", "0.3"));
    s.transactions.push(tx(
        "receipt",
        "rent",
        "2024-03-05",
        &[("eur", "0.3", "principal")],
    ));
    s.cash_links = vec![
        link("one", "p", "p-r", "receipt", 0, "0.1"),
        link("two", "p", "p-r", "receipt", 0, "0.2"),
    ];
    validate_treasury(&r, &p, &l, &s).unwrap();
    assert_eq!(
        account_balance_at(&s, "eur", "2024-03-05")
            .unwrap()
            .unwrap(),
        BigRational::new(3.into(), 10.into())
    );
    s.cash_links[1].amount = "0.20000001".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
}

#[test]
fn unknown_cash_opening_does_not_fabricate_a_zero_or_allow_a_known_negative_balance() {
    let (r, p, l, mut s) = fixture();
    s.transactions.push(tx(
        "unknown-withdrawal",
        "payment",
        "2024-01-02",
        &[("tnd", "-10", "principal")],
    ));
    validate_treasury(&r, &p, &l, &s).unwrap();
    assert_eq!(account_balance_at(&s, "tnd", "2024-01-02").unwrap(), None);
    assert_eq!(
        deposit_applied_at(&s, "none", "rent", "2024-01-02").unwrap(),
        BigRational::zero()
    );
}

fn debt_fixture() -> (
    RegistrySnapshot,
    PropertySnapshot,
    RentalSnapshot,
    TreasurySnapshot,
    DebtSnapshot,
) {
    let (r, p, l, mut s) = fixture();
    let contract = DebtContract {
        id: "loan".into(),
        revision_id: "loan-r".into(),
        name: "Prêt synthétique".into(),
        currency: "EUR".into(),
        tracking_start: "2024-01-01".into(),
        notes: String::new(),
        archived: false,
    };
    let parties = [("creditor", "tenant"), ("debtor", "a")]
        .into_iter()
        .map(|(side, entity)| FinancingParty {
            line_id: format!("{side}-party"),
            side: side.into(),
            entity_id: entity.into(),
            legal_share: Some(f("1", "1")),
            economic_share: Some(f("1", "1")),
            liability_kind: "single".into(),
            notes: String::new(),
        })
        .collect();
    let terms = DebtTerms {
        id: "loan-terms".into(),
        revision_id: "loan-terms-r".into(),
        debt_id: "loan".into(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        principal_status: "complete".into(),
        interest_rate: None,
        interest_convention: String::new(),
        parties,
        notes: String::new(),
    };
    let movements = vec![
        DebtMovement {
            id: "loan-opening".into(),
            revision_id: "loan-opening-r".into(),
            debt_id: "loan".into(),
            kind: "opening".into(),
            effective_on: "2024-01-01".into(),
            principal_change: "100000".into(),
            interest_paid: "0".into(),
            fees_paid: "0".into(),
            unlinked_reason: String::new(),
            notes: String::new(),
        },
        DebtMovement {
            id: "repayment".into(),
            revision_id: "repayment-r".into(),
            debt_id: "loan".into(),
            kind: "principal_repayment".into(),
            effective_on: "2024-01-02".into(),
            principal_change: "-1000".into(),
            interest_paid: "200".into(),
            fees_paid: "10".into(),
            unlinked_reason: "Rapprochement synthétique".into(),
            notes: String::new(),
        },
    ];
    let d = DebtSnapshot {
        contracts: vec![contract],
        terms: vec![terms],
        movements,
        ..Default::default()
    };
    s.transactions = vec![
        tx(
            "open",
            "opening",
            "2024-01-01",
            &[("eur", "100000", "opening")],
        ),
        tx(
            "repayment-cash",
            "loan",
            "2024-01-02",
            &[("eur", "-1210", "principal")],
        ),
    ];
    for (index, (component, amount)) in
        [("principal", "-1000"), ("interest", "-200"), ("fee", "-10")]
            .into_iter()
            .enumerate()
    {
        let mut matched = link(
            &format!("debt-link-{index}"),
            "repayment",
            "repayment-r",
            "repayment-cash",
            0,
            amount,
        );
        matched.source_kind = "debt_movement".into();
        matched.component = component.into();
        matched.side = "debtor".into();
        s.cash_links.push(matched);
    }
    (r, p, l, s, d)
}

#[test]
fn financing_cash_components_match_1210_but_only_1000_reduces_capital() {
    let (r, p, l, s, d) = debt_fixture();
    validate_treasury(&r, &p, &l, &s).unwrap();
    validate_treasury_debt_links(&r, &s, &d).unwrap();
    assert_eq!(
        report(&r, &p, &l, &s, "eur", "2024-01-02")
            .balance_original
            .as_deref(),
        Some("98790.00")
    );
    assert_eq!(
        debt_balance(&r, &d, "loan", "2024-01-02")
            .unwrap()
            .exact_principal,
        Some(f("99000", "1"))
    );
}

#[test]
fn financing_links_cannot_move_fees_to_creditor_or_exceed_a_component() {
    let (r, p, l, mut s, d) = debt_fixture();
    s.cash_links[0].amount = "-1001".into();
    assert!(validate_treasury(&r, &p, &l, &s).is_err());
    assert!(validate_treasury_debt_links(&r, &s, &d).is_err());
    s.cash_links[0].amount = "-1000".into();
    s.cash_links[2].side = "creditor".into();
    assert!(validate_treasury_debt_links(&r, &s, &d).is_err());
    s.cash_links[2].side = "debtor".into();
    s.cash_links[1].source_revision_id = "old-interest-source".into();
    assert!(validate_treasury_debt_links(&r, &s, &d).is_err());
}

#[test]
fn financing_opening_or_unrelated_account_cannot_be_presented_as_repayment_cash() {
    let (r, _, _, mut s, d) = debt_fixture();
    s.accounts[0].legal_owner_entity_id = "tenant".into();
    assert!(validate_treasury_debt_links(&r, &s, &d).is_err());
    s.accounts[0].legal_owner_entity_id = "a".into();
    s.cash_links[0].source_id = "loan-opening".into();
    s.cash_links[0].source_revision_id = "loan-opening-r".into();
    assert!(validate_treasury_debt_links(&r, &s, &d).is_err());
}
