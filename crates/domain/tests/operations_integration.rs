use patrimoine_domain::*;

const START: &str = "2024-01-01";
const DATE: &str = "2024-03-31";
fn f(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
    }
}
fn ownership(asset: &str, owner: &str) -> Ownership {
    Ownership {
        id: format!("own-{asset}"),
        revision_id: format!("own-{asset}-r"),
        asset_id: asset.into(),
        owner_entity_id: owner.into(),
        legal_share: Some(f("1", "1")),
        economic_share: Some(f("1", "1")),
        valid_from: START.into(),
        valid_to: None,
    }
}
fn journal(id: &str, kind: &str, date: &str, rows: &[(&str, &str)]) -> TreasuryTransaction {
    TreasuryTransaction {
        id: id.into(),
        revision_id: format!("{id}-r"),
        kind: kind.into(),
        effective_on: date.into(),
        label: id.into(),
        reference: String::new(),
        notes: String::new(),
        entries: rows
            .iter()
            .enumerate()
            .map(|(i, (account, amount))| TreasuryEntry {
                line_id: format!("line-{i}"),
                account_id: (*account).into(),
                amount: (*amount).into(),
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
struct Fixture {
    registry: RegistrySnapshot,
    homes: PropertySnapshot,
    rentals: RentalSnapshot,
    treasury: TreasurySnapshot,
    debts: DebtSnapshot,
    finance: FinanceSnapshot,
    metals: MetalSnapshot,
    companies: CompanySnapshot,
}
impl Fixture {
    fn new() -> Self {
        let mut registry = RegistrySnapshot {
            knowledge_sequence: 7,
            ..Default::default()
        };
        for (id, kind) in [
            ("a", "person"),
            ("b", "person"),
            ("outside", "person"),
            ("co", "company"),
        ] {
            registry.entities.push(Entity {
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
            });
        }
        for id in ["a", "b"] {
            registry.memberships.push(Membership {
                id: format!("member-{id}"),
                revision_id: format!("member-{id}-r"),
                person_id: id.into(),
                valid_from: START.into(),
                valid_to: None,
                relationship: String::new(),
            });
        }
        registry.participations.push(Participation {
            id: "co-rights".into(),
            revision_id: "co-rights-r".into(),
            company_id: "co".into(),
            owner_entity_id: "a".into(),
            legal_share: Some(f("1", "1")),
            economic_share: Some(f("1", "1")),
            voting_share: Some(f("1", "1")),
            valid_from: START.into(),
            valid_to: None,
        });
        registry.company_policies.push(CompanyPolicy {
            id: "policy".into(),
            revision_id: "policy-r".into(),
            company_id: "co".into(),
            mode: "detailed".into(),
            coverage_complete: true,
            liability_qualified: true,
            valid_from: START.into(),
            valid_to: None,
        });
        for (asset, owner) in [
            ("bank-a-asset", "a"),
            ("bank-co-asset", "co"),
            ("metal-asset", "a"),
        ] {
            registry.assets.push(Asset {
                id: asset.into(),
                revision_id: format!("{asset}-r"),
                name: asset.into(),
                country: "FR".into(),
                category: "other".into(),
                notes: String::new(),
                archived: false,
            });
            registry.ownerships.push(ownership(asset, owner));
        }
        registry.valuations.push(Valuation {
            id: "generic-metal-estimate".into(),
            revision_id: "generic-metal-estimate-r".into(),
            target_kind: "asset".into(),
            target_id: "metal-asset".into(),
            amount: "999999".into(),
            currency: "EUR".into(),
            effective_on: START.into(),
            source: "Must be replaced by actual fine metal stock".into(),
        });
        let mut treasury = TreasurySnapshot {
            knowledge_sequence: 7,
            ..Default::default()
        };
        for (id, owner, balance) in [("bank-a", "a", "100"), ("bank-co", "co", "1000")] {
            treasury.accounts.push(TreasuryAccount {
                id: id.into(),
                revision_id: format!("{id}-r"),
                asset_id: format!("{id}-asset"),
                kind: "bank".into(),
                currency: "EUR".into(),
                tracking_start: START.into(),
                legal_owner_entity_id: owner.into(),
                bank_name: "Test".into(),
                reference: String::new(),
                country: "FR".into(),
                storage_location: String::new(),
                notes: String::new(),
                archived: false,
            });
            treasury.transactions.push(journal(
                &format!("opening-{id}"),
                "opening",
                START,
                &[(id, balance)],
            ));
        }
        let mut metals = MetalSnapshot {
            knowledge_sequence: 7,
            ..Default::default()
        };
        metals.lots.push(MetalLot {
            id: "lot".into(),
            revision_id: "lot-r".into(),
            asset_id: "metal-asset".into(),
            metal: "gold".into(),
            form: "coin".into(),
            designation: "Two identifiable coins".into(),
            unit_weight: "31.1034768".into(),
            weight_unit: "g".into(),
            purity: "0.9".into(),
            tracking_start: START.into(),
            cost_currency: "EUR".into(),
            legal_owner_entity_id: "a".into(),
            country: "FR".into(),
            storage_location: "Test safe".into(),
            valuation_basis: "intrinsic".into(),
            notes: String::new(),
            archived: false,
        });
        metals.movements.push(MetalMovement {
            id: "opening-lot".into(),
            revision_id: "opening-lot-r".into(),
            lot_id: "lot".into(),
            kind: "opening".into(),
            effective_on: START.into(),
            quantity_change: "2".into(),
            cost_amount: Some("2000".into()),
            trade_amount: None,
            trade_fees: "0".into(),
            trade_currency: None,
            cash_entity_id: String::new(),
            counterparty_entity_id: None,
            country: "FR".into(),
            storage_location: "Test safe".into(),
            unlinked_reason: String::new(),
            notes: "Historical stock, no new cash".into(),
        });
        metals.quotes.push(MetalQuote {
            id: "gold-quote".into(),
            revision_id: "gold-quote-r".into(),
            metal: "gold".into(),
            effective_on: START.into(),
            price: "60".into(),
            currency: "EUR".into(),
            weight_unit: "g".into(),
            source: "Independent O10 oracle".into(),
            notes: String::new(),
        });
        Self {
            registry,
            treasury,
            metals,
            homes: PropertySnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            rentals: RentalSnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            debts: DebtSnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            companies: CompanySnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            finance: FinanceSnapshot {
                knowledge_sequence: 7,
                coverages: vec![FinanceCoverage {
                    id: "coverage".into(),
                    revision_id: "coverage-r".into(),
                    valid_from: START.into(),
                    valid_to: None,
                    assets_complete: true,
                    debts_complete: true,
                    rentals_complete: true,
                    notes: "Explicit synthetic inventory".into(),
                }],
                ..Default::default()
            },
        }
    }
    fn report(&self, person: Option<&str>) -> FinancialReport {
        financial_report_with_operations(
            &self.registry,
            &self.homes,
            &self.rentals,
            &self.treasury,
            &self.debts,
            &self.finance,
            &self.metals,
            &self.companies,
            &FinancialReportRequest {
                context: if person.is_some() { "person" } else { "family" }.into(),
                person_id: person.map(str::to_owned),
                date: DATE.into(),
                knowledge_sequence: Some(7),
            },
        )
        .unwrap()
    }
    fn without_metals(&mut self) {
        self.metals = MetalSnapshot {
            knowledge_sequence: 7,
            ..Default::default()
        };
        self.registry.assets.retain(|a| a.id != "metal-asset");
        self.registry
            .ownerships
            .retain(|o| o.asset_id != "metal-asset");
        self.registry.valuations.clear();
    }
    fn sale(&mut self, linked: bool) {
        self.metals.movements.push(MetalMovement {
            id: "sale".into(),
            revision_id: "sale-r".into(),
            lot_id: "lot".into(),
            kind: "sale".into(),
            effective_on: "2024-03-01".into(),
            quantity_change: "-1".into(),
            cost_amount: None,
            trade_amount: Some("1800".into()),
            trade_fees: "10".into(),
            trade_currency: Some("EUR".into()),
            cash_entity_id: "a".into(),
            counterparty_entity_id: Some("outside".into()),
            country: "FR".into(),
            storage_location: "Test safe".into(),
            unlinked_reason: "Receipt explicitly declared; require actual ledger link".into(),
            notes: String::new(),
        });
        if linked {
            self.treasury.transactions.push(journal(
                "sale-cash",
                "asset_trade",
                "2024-03-01",
                &[("bank-a", "1790")],
            ));
            self.metals.cash_links.push(MetalCashLink {
                id: "sale-link".into(),
                revision_id: "sale-link-r".into(),
                movement_id: "sale".into(),
                movement_revision_id: "sale-r".into(),
                transaction_revision_id: "sale-cash-r".into(),
                line_id: "line-0".into(),
                amount: "1790".into(),
                notes: String::new(),
            });
        }
    }
    fn distribution(&mut self, links: &[&str]) {
        self.treasury.transactions.push(journal(
            "distribution-cash",
            "dividend",
            "2024-03-01",
            &[("bank-co", "-500"), ("bank-a", "500")],
        ));
        self.companies.distributions.push(CompanyDistribution {
            id: "paid-dividend".into(),
            revision_id: "paid-dividend-r".into(),
            company_id: "co".into(),
            beneficiary_entity_id: "a".into(),
            kind: "paid".into(),
            effective_on: "2024-03-01".into(),
            amount: "500".into(),
            currency: "EUR".into(),
            related_declaration_id: None,
            unlinked_reason: "Missing source sides remain explicit".into(),
            notes: String::new(),
            cash_links: links
                .iter()
                .map(|side| CompanyDistributionCashLink {
                    side: (*side).into(),
                    transaction_revision_id: "distribution-cash-r".into(),
                    line_id: if *side == "payer" { "line-0" } else { "line-1" }.into(),
                    amount: if *side == "payer" { "-500" } else { "500" }.into(),
                })
                .collect(),
        });
    }
}

#[test]
fn metal_exact_stock_replaces_generic_estimate_and_is_rounded_only_at_the_end() {
    let s = Fixture::new();
    let r = s.report(None);
    assert_eq!(r.net_eur.as_deref(), Some("4459.18"));
    assert_eq!(r.exact_net, Some(f("696746171", "156250")));
    assert_eq!(r.positions.iter().filter(|p| p.kind == "metal").count(), 1);
    assert!(!r.source_ids.iter().any(|id| id == "generic-metal-estimate"));
    let c = consolidate_with_metals(
        &s.registry,
        &s.homes,
        &s.treasury,
        &s.metals,
        &ConsolidationRequest {
            context: "family".into(),
            person_id: None,
            date: DATE.into(),
            knowledge_sequence: Some(7),
        },
    )
    .unwrap();
    assert_eq!(c.gross_eur, "3359.18");
    assert_eq!(c.exact_gross, f("524871171", "156250"));
    // The legacy subtotal still flags the fixture's unestimated bank assets.
    // Isolate the physical stock to verify its own coverage after replacement.
    let mut stock_registry = s.registry.clone();
    stock_registry.assets.retain(|a| a.id == "metal-asset");
    stock_registry
        .ownerships
        .retain(|o| o.asset_id == "metal-asset");
    let stock_only = consolidate_with_metals(
        &stock_registry,
        &s.homes,
        &TreasurySnapshot {
            knowledge_sequence: 7,
            ..Default::default()
        },
        &s.metals,
        &ConsolidationRequest {
            context: "family".into(),
            person_id: None,
            date: DATE.into(),
            knowledge_sequence: Some(7),
        },
    )
    .unwrap();
    assert!(stock_only.gross_is_underlying_complete);
}
#[test]
fn future_quote_or_unknown_opening_does_not_become_a_complete_metal_stock() {
    let mut s = Fixture::new();
    s.metals.quotes[0].effective_on = "2025-01-01".into();
    assert_eq!(s.report(None).net_eur, None);
    s.metals.quotes[0].effective_on = START.into();
    s.metals.movements.clear();
    assert_eq!(s.report(None).net_eur, None);
}
#[test]
fn resale_premium_is_an_explicit_method_and_unknown_premium_does_not_mean_zero() {
    let mut s = Fixture::new();
    s.metals.lots[0].valuation_basis = "resale".into();
    assert_eq!(s.report(None).net_eur, None);
    s.metals.premiums.push(MetalPremium {
        id: "premium".into(),
        revision_id: "premium-r".into(),
        lot_id: "lot".into(),
        effective_on: START.into(),
        kind: "amount_per_unit".into(),
        value: "50".into(),
        currency: "EUR".into(),
        discount_per_unit: "10".into(),
        source: "Explicit per coin premium and discount".into(),
        notes: String::new(),
    });
    assert_eq!(s.report(None).net_eur.as_deref(), Some("4539.18"));
}
#[test]
fn partial_sale_has_one_net_cash_receipt_and_remaining_homogeneous_cost() {
    let mut s = Fixture::new();
    s.sale(true);
    assert_eq!(s.report(None).net_eur.as_deref(), Some("4569.59"));
    let r = metal_report(
        &s.registry,
        &s.homes,
        &s.treasury,
        &s.metals,
        &MetalReportRequest {
            lot_id: "lot".into(),
            date: DATE.into(),
            knowledge_sequence: Some(7),
        },
    )
    .unwrap();
    assert_eq!(r.cost_remaining_original.as_deref(), Some("1000.00"));
    assert_eq!(r.realized_gain_eur.as_deref(), Some("790.00"));
}
#[test]
fn an_unlinked_metal_sale_reduces_stock_but_does_not_invent_a_bank_receipt() {
    let mut s = Fixture::new();
    s.sale(false);
    let r = s.report(None);
    assert_eq!(r.net_eur, None);
    assert_eq!(r.known_assets_eur, "2779.59");
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "unlinked_metal_trade_cash"));
}
#[test]
fn owner_transfer_changes_individual_composition_without_changing_family_stock() {
    let mut s = Fixture::new();
    let before = s.report(None).exact_net;
    s.registry
        .ownerships
        .iter_mut()
        .find(|o| o.asset_id == "metal-asset")
        .unwrap()
        .owner_entity_id = "b".into();
    assert_eq!(s.report(None).exact_net, before);
    assert_eq!(s.report(Some("a")).net_eur.as_deref(), Some("1100.00"));
    assert_eq!(s.report(Some("b")).net_eur.as_deref(), Some("3359.18"));
}
#[test]
fn an_opaque_company_keeps_its_metal_inside_equity_instead_of_counting_it_again() {
    let mut s = Fixture::new();
    s.registry
        .ownerships
        .iter_mut()
        .find(|o| o.asset_id == "metal-asset")
        .unwrap()
        .owner_entity_id = "co".into();
    s.registry.company_policies[0].mode = "global_equity".into();
    s.registry.valuations.push(Valuation {
        id: "equity".into(),
        revision_id: "equity-r".into(),
        target_kind: "company".into(),
        target_id: "co".into(),
        amount: "5000".into(),
        currency: "EUR".into(),
        effective_on: START.into(),
        source: "All assets and debt included".into(),
    });
    let r = s.report(None);
    assert_eq!(r.net_eur.as_deref(), Some("5100.00"));
    assert!(!r.positions.iter().any(|p| p.kind == "metal"));
}
#[test]
fn indicative_company_cash_and_declared_dividend_are_not_extra_financial_positions() {
    let mut s = Fixture::new();
    s.without_metals();
    let before = s.report(None).exact_net;
    s.companies.financial_periods.push(CompanyFinancialPeriod {
        id: "period".into(),
        revision_id: "period-r".into(),
        company_id: "co".into(),
        period_start: START.into(),
        period_end: "2024-02-01".into(),
        frequency: "monthly".into(),
        currency: "EUR".into(),
        revenue: Some("10000".into()),
        operating_expenses: Some("5000".into()),
        operating_result: Some("5000".into()),
        net_result: Some("4000".into()),
        indicative_cash: Some("999999".into()),
        indicative_debt: Some("700000".into()),
        important_receivables: Some("800000".into()),
        dividends_declared: Some("500".into()),
        indicative_global_value: Some("1000000".into()),
        notes: String::new(),
    });
    s.companies.distributions.push(CompanyDistribution {
        id: "declared".into(),
        revision_id: "declared-r".into(),
        company_id: "co".into(),
        beneficiary_entity_id: "a".into(),
        kind: "declared".into(),
        effective_on: "2024-02-01".into(),
        amount: "500".into(),
        currency: "EUR".into(),
        related_declaration_id: None,
        unlinked_reason: String::new(),
        notes: String::new(),
        cash_links: vec![],
    });
    assert_eq!(s.report(None).exact_net, before);
    assert_eq!(s.report(None).net_eur.as_deref(), Some("1100.00"));
}
#[test]
fn a_paid_dividend_needs_every_visible_cash_side_before_a_complete_net() {
    for sides in [vec![], vec!["payer"], vec!["beneficiary"]] {
        let mut s = Fixture::new();
        s.without_metals();
        s.distribution(&sides);
        let r = s.report(None);
        assert_eq!(r.net_eur, None);
        assert!(r
            .warnings
            .iter()
            .any(|w| w.code == "unlinked_dividend_cash"));
    }
    let mut s = Fixture::new();
    s.without_metals();
    s.distribution(&["payer", "beneficiary"]);
    assert_eq!(s.report(None).net_eur.as_deref(), Some("1100.00"));
}
#[test]
fn a_line_cannot_be_spent_again_by_a_dividend_after_a_metal_cash_link() {
    let mut s = Fixture::new();
    s.sale(true);
    s.companies.distributions.push(CompanyDistribution {
        id: "paid-dividend".into(),
        revision_id: "paid-dividend-r".into(),
        company_id: "co".into(),
        beneficiary_entity_id: "a".into(),
        kind: "paid".into(),
        effective_on: "2024-03-01".into(),
        amount: "1790".into(),
        currency: "EUR".into(),
        related_declaration_id: None,
        unlinked_reason: "Only beneficiary linked".into(),
        notes: String::new(),
        cash_links: vec![CompanyDistributionCashLink {
            side: "beneficiary".into(),
            transaction_revision_id: "sale-cash-r".into(),
            line_id: "line-0".into(),
            amount: "1790".into(),
        }],
    });
    assert!(validate_operation_cash_capacity(&s.treasury, &s.metals, &s.companies).is_err());
}

#[test]
fn an_opaque_company_needs_an_equity_value_that_reflects_its_actual_distribution() {
    let mut s = Fixture::new();
    s.without_metals();
    s.registry.company_policies[0].mode = "global_equity".into();
    s.registry.valuations.push(Valuation {
        id: "equity-before".into(),
        revision_id: "equity-before-r".into(),
        target_kind: "company".into(),
        target_id: "co".into(),
        amount: "5000".into(),
        currency: "EUR".into(),
        effective_on: START.into(),
        source: "Equity before distribution".into(),
    });
    assert_eq!(s.report(None).net_eur.as_deref(), Some("5100.00"));
    s.distribution(&["payer", "beneficiary"]);
    let unknown = s.report(None);
    assert_eq!(unknown.net_eur, None);
    assert!(unknown
        .warnings
        .iter()
        .any(|w| w.code == "opaque_equity_before_distribution"));
    s.registry.valuations.push(Valuation {
        id: "equity-after".into(),
        revision_id: "equity-after-r".into(),
        target_kind: "company".into(),
        target_id: "co".into(),
        amount: "4500".into(),
        currency: "EUR".into(),
        effective_on: "2024-03-01".into(),
        source: "Equity explicitly updated after 500 distribution".into(),
    });
    assert_eq!(s.report(None).net_eur.as_deref(), Some("5100.00"));
}

#[test]
fn an_explicitly_empty_metal_stock_does_not_need_a_nonexistent_foreign_exchange_rate() {
    let mut s = Fixture::new();
    s.metals.lots[0].cost_currency = "USD".into();
    s.metals.movements[0].quantity_change = "0".into();
    s.metals.movements[0].cost_amount = Some("0".into());
    s.metals.quotes.clear();
    assert_eq!(s.report(None).net_eur.as_deref(), Some("1100.00"));
}

#[test]
fn a_metal_transfer_out_of_opaque_equity_needs_a_value_after_the_transfer() {
    let mut s = Fixture::new();
    s.metals.lots[0].unit_weight = "1".into();
    s.metals.lots[0].purity = "1".into();
    s.registry.company_policies[0].mode = "global_equity".into();
    let right = s
        .registry
        .ownerships
        .iter_mut()
        .find(|o| o.asset_id == "metal-asset")
        .unwrap();
    right.owner_entity_id = "co".into();
    right.valid_to = Some("2024-03-01".into());
    let mut new = ownership("metal-asset", "a");
    new.id = "transferred-right".into();
    new.revision_id = "transferred-right-r".into();
    new.valid_from = "2024-03-01".into();
    s.registry.ownerships.push(new);
    s.registry.valuations.push(Valuation {
        id: "equity-before".into(),
        revision_id: "equity-before-r".into(),
        target_kind: "company".into(),
        target_id: "co".into(),
        amount: "5000".into(),
        currency: "EUR".into(),
        effective_on: START.into(),
        source: "Equity including the 120 metal lot".into(),
    });
    let unknown = s.report(None);
    assert_eq!(unknown.net_eur, None);
    assert!(unknown
        .warnings
        .iter()
        .any(|w| w.code == "opaque_equity_before_metal_transfer"));
    s.registry.valuations.push(Valuation {
        id: "equity-after".into(),
        revision_id: "equity-after-r".into(),
        target_kind: "company".into(),
        target_id: "co".into(),
        amount: "4880".into(),
        currency: "EUR".into(),
        effective_on: "2024-03-01".into(),
        source: "Equity explicitly excluding the transferred 120 metal lot".into(),
    });
    assert_eq!(s.report(None).net_eur.as_deref(), Some("5100.00"));
}

#[test]
fn a_purchase_paid_by_an_opaque_company_for_a_visible_lot_does_not_certify_a_false_gain() {
    let mut s = Fixture::new();
    s.registry.company_policies[0].mode = "global_equity".into();
    s.metals.movements[0].quantity_change = "0".into();
    s.metals.movements[0].cost_amount = Some("0".into());
    s.registry.valuations.push(Valuation {
        id: "equity-before".into(),
        revision_id: "equity-before-r".into(),
        target_kind: "company".into(),
        target_id: "co".into(),
        amount: "5000".into(),
        currency: "EUR".into(),
        effective_on: START.into(),
        source: "Equity before cash was used outside its opaque perimeter".into(),
    });
    s.metals.movements.push(MetalMovement {
        id: "purchase".into(),
        revision_id: "purchase-r".into(),
        lot_id: "lot".into(),
        kind: "purchase".into(),
        effective_on: "2024-03-01".into(),
        quantity_change: "1".into(),
        cost_amount: Some("500".into()),
        trade_amount: Some("500".into()),
        trade_fees: "0".into(),
        trade_currency: Some("EUR".into()),
        cash_entity_id: "co".into(),
        counterparty_entity_id: Some("outside".into()),
        country: "FR".into(),
        storage_location: "Test safe".into(),
        unlinked_reason: String::new(),
        notes: String::new(),
    });
    s.treasury.transactions.push(journal(
        "purchase-cash",
        "asset_trade",
        "2024-03-01",
        &[("bank-co", "-500")],
    ));
    s.metals.cash_links.push(MetalCashLink {
        id: "purchase-link".into(),
        revision_id: "purchase-link-r".into(),
        movement_id: "purchase".into(),
        movement_revision_id: "purchase-r".into(),
        transaction_revision_id: "purchase-cash-r".into(),
        line_id: "line-0".into(),
        amount: "-500".into(),
        notes: String::new(),
    });
    let r = s.report(None);
    assert_eq!(r.net_eur, None);
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "opaque_equity_before_metal_operation"));
}
