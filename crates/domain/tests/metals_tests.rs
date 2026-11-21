use patrimoine_domain::*;

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
        category: "metal".into(),
        notes: String::new(),
        archived: false,
    }
}
fn movement(id: &str, kind: &str, quantity: &str, date: &str, cost: Option<&str>) -> MetalMovement {
    let trade = kind == "purchase" || kind == "sale";
    MetalMovement {
        id: id.into(),
        revision_id: format!("{id}-r"),
        lot_id: "lot".into(),
        kind: kind.into(),
        effective_on: date.into(),
        quantity_change: quantity.into(),
        cost_amount: cost.map(str::to_owned),
        trade_amount: if trade { Some("1800".into()) } else { None },
        trade_fees: if trade { "10".into() } else { "0".into() },
        trade_currency: if trade { Some("EUR".into()) } else { None },
        cash_entity_id: if trade { "alice".into() } else { String::new() },
        counterparty_entity_id: None,
        country: "FR".into(),
        storage_location: "Coffre A".into(),
        unlinked_reason: "Sans lien bancaire dans cette fixture analytique".into(),
        notes: String::new(),
    }
}
struct Fixture {
    r: RegistrySnapshot,
    p: PropertySnapshot,
    t: TreasurySnapshot,
    m: MetalSnapshot,
}
fn fixture() -> Fixture {
    let r = RegistrySnapshot {
        entities: vec![person("alice"), person("bob")],
        assets: vec![asset("gold"), asset("bank-asset")],
        memberships: vec![],
        participations: vec![],
        ownerships: vec![],
        valuations: vec![],
        company_policies: vec![],
        exchange_rates: vec![],
        profile_periods: vec![],
        profile_revisions: vec![],
        asset_profile_revisions: vec![],
        knowledge_sequence: 8,
    };
    let m = MetalSnapshot {
        lots: vec![MetalLot {
            id: "lot".into(),
            revision_id: "lot-r".into(),
            asset_id: "gold".into(),
            metal: "gold".into(),
            form: "coin".into(),
            designation: "Deux pièces identifiées".into(),
            unit_weight: "1".into(),
            weight_unit: "troy_oz".into(),
            purity: "0.9".into(),
            tracking_start: "2024-01-01".into(),
            cost_currency: "EUR".into(),
            legal_owner_entity_id: "alice".into(),
            country: "FR".into(),
            storage_location: "Coffre A".into(),
            valuation_basis: "resale".into(),
            notes: String::new(),
            archived: false,
        }],
        movements: vec![movement(
            "opening",
            "opening",
            "2",
            "2024-01-01",
            Some("2400"),
        )],
        quotes: vec![MetalQuote {
            id: "quote".into(),
            revision_id: "quote-r".into(),
            metal: "gold".into(),
            effective_on: "2024-03-01".into(),
            price: "60".into(),
            currency: "EUR".into(),
            weight_unit: "g".into(),
            source: "Cotation synthétique".into(),
            notes: String::new(),
        }],
        premiums: vec![MetalPremium {
            id: "premium".into(),
            revision_id: "premium-r".into(),
            lot_id: "lot".into(),
            effective_on: "2024-03-01".into(),
            kind: "amount_per_unit".into(),
            value: "10".into(),
            currency: "EUR".into(),
            discount_per_unit: "5".into(),
            source: "Prime et frais déclarés".into(),
            notes: String::new(),
        }],
        cash_links: vec![],
        knowledge_sequence: 8,
    };
    Fixture {
        r,
        p: PropertySnapshot {
            knowledge_sequence: 8,
            ..Default::default()
        },
        t: TreasurySnapshot {
            knowledge_sequence: 8,
            ..Default::default()
        },
        m,
    }
}
impl Fixture {
    fn report(&self, date: &str) -> MetalReport {
        metal_report(
            &self.r,
            &self.p,
            &self.t,
            &self.m,
            &MetalReportRequest {
                lot_id: "lot".into(),
                date: date.into(),
                knowledge_sequence: Some(8),
            },
        )
        .unwrap()
    }
    fn valid(&self) -> bool {
        validate_metals(&self.r, &self.p, &self.t, &self.m).is_ok()
    }
    fn fx(&mut self, id: &str, rate: &str, date: &str) {
        self.r.exchange_rates.push(ExchangeRate {
            id: id.into(),
            revision_id: format!("{id}-r"),
            base_currency: "USD".into(),
            quote_currency: "EUR".into(),
            rate: rate.into(),
            effective_on: date.into(),
            source: Some("Source historique".into()),
            notes: String::new(),
        });
    }
    fn sale(&mut self) {
        self.m
            .movements
            .push(movement("sale", "sale", "-1", "2024-03-10", None));
    }
    fn bank_sale(&mut self) {
        self.t.accounts.push(TreasuryAccount {
            id: "bank".into(),
            revision_id: "bank-r".into(),
            asset_id: "bank-asset".into(),
            kind: "bank".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            legal_owner_entity_id: "alice".into(),
            bank_name: "Banque".into(),
            reference: String::new(),
            country: "FR".into(),
            storage_location: String::new(),
            notes: String::new(),
            archived: false,
        });
        self.t.transactions.push(TreasuryTransaction {
            id: "receipt".into(),
            revision_id: "receipt-r".into(),
            kind: "asset_trade".into(),
            effective_on: "2024-03-10".into(),
            label: "Produit de vente net".into(),
            reference: String::new(),
            notes: String::new(),
            entries: vec![TreasuryEntry {
                line_id: "receipt-line".into(),
                account_id: "bank".into(),
                amount: "1790".into(),
                role: "principal".into(),
            }],
        });
        self.m.cash_links.push(MetalCashLink {
            id: "sale-link".into(),
            revision_id: "sale-link-r".into(),
            movement_id: "sale".into(),
            movement_revision_id: "sale-r".into(),
            transaction_revision_id: "receipt-r".into(),
            line_id: "receipt-line".into(),
            amount: "1790".into(),
            notes: String::new(),
        });
    }
}

#[test]
fn o10_exact_fine_weight_intrinsic_premium_and_resale_are_separate() {
    let s = fixture();
    let r = s.report("2024-03-31");
    assert_eq!(r.quantity_remaining.as_deref(), Some("2"));
    assert_eq!(r.fine_weight_g.as_deref(), Some("55.98625824"));
    assert_eq!(
        r.exact_intrinsic_original,
        Some(Fraction {
            num: "524871171".into(),
            den: "156250".into()
        })
    );
    assert_eq!(r.intrinsic_eur.as_deref(), Some("3359.18"));
    assert_eq!(r.premium_eur.as_deref(), Some("20.00"));
    assert_eq!(r.discount_eur.as_deref(), Some("10.00"));
    assert_eq!(r.resale_eur.as_deref(), Some("3369.18"));
    assert_eq!(r.cost_remaining_original.as_deref(), Some("2400.00"));
    assert_eq!(r.unrealized_gain_eur.as_deref(), Some("969.18"));
}

#[test]
fn o12_kilogram_and_troy_ounce_are_exact_and_ordinary_ounce_is_rejected() {
    assert_eq!(
        exact::fraction_of(&metal_weight_in_grams("1", "kg").unwrap()),
        Fraction {
            num: "1000".into(),
            den: "1".into()
        }
    );
    assert_eq!(
        exact::fraction_of(&metal_weight_in_grams("1", "troy_oz").unwrap()),
        Fraction {
            num: "19439673".into(),
            den: "625000".into()
        }
    );
    assert!(metal_weight_in_grams("1", "oz").is_err());
    assert!(metal_weight_in_grams("-1", "g").is_err());
}

#[test]
fn o11_partial_sale_removes_proportional_cost_and_bank_receipt_is_not_stock() {
    let mut s = fixture();
    s.sale();
    s.bank_sale();
    let r = s.report("2024-03-31");
    assert_eq!(r.quantity_remaining.as_deref(), Some("1"));
    assert_eq!(r.fine_weight_g.as_deref(), Some("27.99312912"));
    assert_eq!(r.cost_remaining_eur.as_deref(), Some("1200.00"));
    assert_eq!(r.realized_gain_eur.as_deref(), Some("590.00"));
    assert_eq!(r.resale_eur.as_deref(), Some("1684.59"));
    assert_eq!(
        r.cost_allocations[0].exact_cost_original,
        Some(Fraction {
            num: "1200".into(),
            den: "1".into()
        })
    );
    assert!(metal_cash_coverage(&s.m, &s.m.movements[1]).unwrap());
    assert!(!r.warnings.iter().any(|w| w.code == "unlinked_metal_cash"));
}

#[test]
fn additional_purchase_uses_identified_lot_proportions_without_global_fifo() {
    let mut s = fixture();
    s.m.movements.push(movement(
        "purchase",
        "purchase",
        "1",
        "2024-02-01",
        Some("1800"),
    ));
    s.sale();
    let r = s.report("2024-03-31");
    assert_eq!(r.quantity_remaining.as_deref(), Some("2"));
    assert_eq!(r.cost_remaining_original.as_deref(), Some("2800.00"));
    assert_eq!(
        r.cost_allocations[0].exact_cost_original,
        Some(Fraction {
            num: "1400".into(),
            den: "1".into()
        })
    );
    assert_eq!(r.realized_gain_eur.as_deref(), Some("390.00"));
}

#[test]
fn costs_use_historical_fx_while_remaining_stock_uses_observation_quote() {
    let mut s = fixture();
    s.m.lots[0].cost_currency = "USD".into();
    s.fx("old", "0.8", "2024-01-01");
    s.fx("current", "0.9", "2024-03-01");
    let r = s.report("2024-03-31");
    assert_eq!(r.cost_remaining_original.as_deref(), Some("2400.00"));
    assert_eq!(r.cost_remaining_eur.as_deref(), Some("1920.00"));
    assert_eq!(r.resale_eur.as_deref(), Some("3369.18"));
    assert!(r.source_ids.contains(&"old-r".into()));
}

#[test]
fn missing_cost_does_not_hide_known_stock_value_and_missing_premium_is_not_zero() {
    let mut s = fixture();
    s.m.movements[0].cost_amount = None;
    s.m.premiums.clear();
    s.m.lots[0].valuation_basis = "intrinsic".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.cost_remaining_original, None);
    assert_eq!(r.resale_eur, None);
    assert_eq!(r.valuation_eur.as_deref(), Some("3359.18"));
    assert!(r.warnings.iter().any(|w| w.code == "missing_metal_cost"));
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "missing_metal_resale_terms"));
    s.m.lots[0].valuation_basis = "resale".into();
    assert_eq!(s.report("2024-03-31").valuation_eur, None);
}

#[test]
fn absent_opening_unknown_and_confirmed_zero_is_zero_without_future_quotes() {
    let mut s = fixture();
    s.m.lots[0].cost_currency = "USD".into();
    s.m.movements.clear();
    s.m.quotes.clear();
    s.m.premiums.clear();
    assert_eq!(s.report("2024-03-31").quantity_remaining, None);
    s.m.movements
        .push(movement("zero", "opening", "0", "2024-01-01", None));
    let r = s.report("2024-03-31");
    assert_eq!(r.quantity_remaining.as_deref(), Some("0"));
    assert_eq!(r.resale_eur.as_deref(), Some("0.00"));
    assert_eq!(r.currency, "EUR");
    assert_eq!(r.cost_currency, "USD");
    assert_eq!(s.report("2023-12-31").quantity_remaining, None);
}

#[test]
fn future_quote_and_future_premium_are_never_applied_to_the_past() {
    let mut s = fixture();
    s.m.quotes[0].effective_on = "2024-04-01".into();
    s.m.premiums[0].effective_on = "2024-04-01".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.intrinsic_eur, None);
    assert_eq!(r.resale_eur, None);
    assert!(r.quote.is_none());
}

#[test]
fn storage_and_legal_owner_profile_do_not_change_stock_or_create_gain() {
    let mut s = fixture();
    let before = s.report("2024-03-31");
    let mut storage = movement("storage", "storage_transfer", "0", "2024-03-20", None);
    storage.country = "CH".into();
    storage.storage_location = "Coffre B".into();
    s.m.movements.push(storage);
    s.m.lots[0].legal_owner_entity_id = "bob".into();
    let after = s.report("2024-03-31");
    assert_eq!(before.exact_quantity, after.exact_quantity);
    assert_eq!(before.exact_valuation_eur, after.exact_valuation_eur);
    assert_eq!(before.realized_gain_eur, after.realized_gain_eur);
    assert_eq!(after.country, "CH");
    assert_eq!(after.storage_location, "Coffre B");
}

#[test]
fn retroactive_oversale_and_unknown_opening_sales_are_rejected() {
    let mut s = fixture();
    s.sale();
    s.m.movements[1].quantity_change = "-3".into();
    assert!(!s.valid());
    s.m.movements[1].quantity_change = "-1".into();
    s.m.movements.remove(0);
    assert!(!s.valid());
}

#[test]
fn sole_current_exact_source_and_ledger_revisions_can_be_linked() {
    let mut s = fixture();
    s.sale();
    s.bank_sale();
    assert!(s.valid());
    s.m.cash_links[0].movement_revision_id = "old-sale-r".into();
    assert!(!s.valid());
    s.m.cash_links[0].movement_revision_id = "sale-r".into();
    s.m.cash_links[0].transaction_revision_id = "old-ledger-r".into();
    assert!(!s.valid());
}

#[test]
fn metal_cash_links_check_date_currency_holder_sign_and_shared_capacity() {
    let original = {
        let mut s = fixture();
        s.sale();
        s.bank_sale();
        s
    };
    let mut s = fixture();
    s.sale();
    s.bank_sale();
    s.t.transactions[0].effective_on = "2024-03-11".into();
    assert!(!s.valid());
    s.t = original.t.clone();
    s.t.accounts[0].currency = "USD".into();
    assert!(!s.valid());
    s.t = original.t.clone();
    s.t.accounts[0].legal_owner_entity_id = "bob".into();
    assert!(!s.valid());
    s.t = original.t.clone();
    s.m.cash_links[0].amount = "-1790".into();
    assert!(!s.valid());
    s.m = original.m.clone();
    s.t.cash_links.push(CashLink {
        id: "other-source".into(),
        revision_id: "other-source-r".into(),
        source_kind: "rent_payment".into(),
        source_id: "payment".into(),
        source_revision_id: "payment-r".into(),
        transaction_revision_id: "receipt-r".into(),
        line_id: "receipt-line".into(),
        amount: "1".into(),
        component: "full".into(),
        side: String::new(),
        notes: String::new(),
    });
    assert!(!s.valid());
}

#[test]
fn source_trade_cash_coverage_is_net_of_fees_once_and_partial_is_explicit() {
    let mut s = fixture();
    s.sale();
    s.bank_sale();
    s.m.cash_links[0].amount = "1700".into();
    assert!(s.valid());
    assert!(!metal_cash_coverage(&s.m, &s.m.movements[1]).unwrap());
    assert!(s
        .report("2024-03-31")
        .warnings
        .iter()
        .any(|w| w.code == "unlinked_metal_cash"));
    s.m.cash_links[0].amount = "1790".into();
    assert!(metal_cash_coverage(&s.m, &s.m.movements[1]).unwrap());
    let buy = movement("buy", "purchase", "1", "2024-03-10", Some("1810"));
    assert_eq!(
        exact::fraction_of(&metal_movement_cash_amount(&buy).unwrap().unwrap()),
        Fraction {
            num: "-1810".into(),
            den: "1".into()
        }
    );
}

#[test]
fn purity_unit_identity_and_snapshot_knowledge_are_validated() {
    let mut s = fixture();
    s.m.lots[0].purity = "90".into();
    assert!(!s.valid());
    s.m.lots[0].purity = "0.9".into();
    s.m.lots[0].weight_unit = "ounce".into();
    assert!(!s.valid());
    s.m.lots[0].weight_unit = "troy_oz".into();
    s.t.knowledge_sequence = 7;
    assert!(!s.valid());
}

#[test]
fn fraction_premium_and_explicit_discount_convert_without_intermediate_rounding() {
    let mut s = fixture();
    s.m.premiums[0].kind = "fraction_of_intrinsic".into();
    s.m.premiums[0].value = "0.1".into();
    s.m.premiums[0].discount_per_unit = "0".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.premium_eur.as_deref(), Some("335.92"));
    assert_eq!(r.resale_eur.as_deref(), Some("3695.09"));
    s.m.premiums[0].discount_per_unit = "9999".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.resale_eur, None);
    assert!(r.warnings.iter().any(|w| w.code == "negative_metal_resale"));
}

#[test]
fn exhausting_an_unknown_cost_lot_leaves_known_zero_stock_without_forged_realized_gain() {
    let mut s = fixture();
    s.m.movements[0].cost_amount = None;
    s.sale();
    s.m.movements[1].quantity_change = "-2".into();
    s.m.quotes.clear();
    s.m.premiums.clear();
    let r = s.report("2024-03-31");
    assert_eq!(r.cost_remaining_eur.as_deref(), Some("0.00"));
    assert_eq!(r.valuation_eur.as_deref(), Some("0.00"));
    assert_eq!(r.realized_gain_eur, None);
}

#[test]
fn zero_stock_cannot_hide_a_nonzero_opening_cost() {
    let mut s = fixture();
    s.m.movements[0].quantity_change = "0".into();
    assert!(!s.valid());
    s.m.movements[0].cost_amount = Some("0".into());
    assert!(s.valid());
}

#[test]
fn changed_storage_requires_explicit_whole_lot_transfer() {
    let mut s = fixture();
    s.sale();
    s.m.movements[1].country = "CH".into();
    s.m.movements[1].storage_location = "Coffre B".into();
    assert!(!s.valid());
    let mut storage = movement("storage", "storage_transfer", "0", "2024-03-01", None);
    storage.country = "CH".into();
    storage.storage_location = "Coffre B".into();
    s.m.movements.push(storage);
    assert!(s.valid());
    assert_eq!(s.report("2024-03-31").storage_location, "Coffre B");
}

#[test]
fn premium_currency_is_converted_into_quote_units_before_one_final_euro_conversion() {
    let mut s = fixture();
    s.m.quotes[0].currency = "USD".into();
    s.fx("usd", "0.5", "2024-01-01");
    let r = s.report("2024-03-31");
    assert_eq!(r.currency, "USD");
    assert_eq!(r.intrinsic_original.as_deref(), Some("3359.18"));
    assert_eq!(r.premium_original.as_deref(), Some("40.00"));
    assert_eq!(r.discount_original.as_deref(), Some("20.00"));
    assert_eq!(r.premium_eur.as_deref(), Some("20.00"));
    assert_eq!(r.discount_eur.as_deref(), Some("10.00"));
    assert_eq!(r.resale_eur.as_deref(), Some("1689.59"));
}

#[test]
fn declared_zero_cost_and_resale_adjustments_do_not_require_an_unneeded_fx_rate() {
    let mut s = fixture();
    s.m.lots[0].cost_currency = "USD".into();
    s.m.movements[0].cost_amount = Some("0".into());
    s.m.premiums[0].currency = "CHF".into();
    s.m.premiums[0].value = "0".into();
    s.m.premiums[0].discount_per_unit = "0".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.cost_remaining_eur.as_deref(), Some("0.00"));
    assert_eq!(r.premium_eur.as_deref(), Some("0.00"));
    assert_eq!(r.discount_eur.as_deref(), Some("0.00"));
    assert_eq!(r.resale_eur.as_deref(), Some("3359.18"));
}

#[test]
fn monetary_adjustments_without_a_quote_use_their_own_euro_conversion() {
    let mut s = fixture();
    s.m.quotes.clear();
    s.m.lots[0].cost_currency = "USD".into();
    s.m.movements[0].cost_amount = Some("0".into());
    s.m.premiums[0].currency = "USD".into();
    let missing_fx = s.report("2024-03-31");
    assert_eq!(missing_fx.premium_original.as_deref(), Some("20.00"));
    assert_eq!(missing_fx.discount_original.as_deref(), Some("10.00"));
    assert_eq!(missing_fx.premium_eur, None);
    assert_eq!(missing_fx.discount_eur, None);
    assert!(missing_fx
        .warnings
        .iter()
        .any(|w| w.code == "missing_exchange_rate" && w.object_id == "premium"));

    s.fx("usd", "0.5", "2024-03-01");
    let r = s.report("2024-03-31");
    assert_eq!(r.premium_eur.as_deref(), Some("10.00"));
    assert_eq!(r.discount_eur.as_deref(), Some("5.00"));
    assert_eq!(r.intrinsic_eur, None);
    assert_eq!(r.resale_eur, None);
    assert!(r.source_ids.contains(&"usd-r".into()));
    assert!(r.conversions.iter().any(|fx| {
        fx.from_currency == "USD"
            && fx.to_currency == "EUR"
            && fx.legs.iter().any(|leg| leg.rate_id == "usd")
    }));
}

#[test]
fn euro_adjustments_remain_known_when_a_foreign_quote_cannot_be_converted() {
    let mut s = fixture();
    s.m.quotes[0].currency = "USD".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.intrinsic_eur, None);
    assert_eq!(r.premium_original, None);
    assert_eq!(r.discount_original, None);
    assert_eq!(r.premium_eur.as_deref(), Some("20.00"));
    assert_eq!(r.discount_eur.as_deref(), Some("10.00"));
    assert_eq!(r.resale_eur, None);
}

#[test]
fn proportional_premium_requires_intrinsic_but_explicit_zeros_do_not_require_fx() {
    let mut s = fixture();
    s.m.quotes.clear();
    s.m.premiums[0].kind = "fraction_of_intrinsic".into();
    s.m.premiums[0].value = "0.1".into();
    s.m.premiums[0].currency = "USD".into();
    s.fx("usd", "0.5", "2024-03-01");
    let r = s.report("2024-03-31");
    assert_eq!(r.premium_eur, None);
    assert_eq!(r.discount_eur.as_deref(), Some("5.00"));
    assert_eq!(r.resale_eur, None);

    s.r.exchange_rates.clear();
    s.m.premiums[0].value = "0".into();
    s.m.premiums[0].discount_per_unit = "0".into();
    let zero = s.report("2024-03-31");
    assert_eq!(zero.premium_eur.as_deref(), Some("0.00"));
    assert_eq!(zero.discount_eur.as_deref(), Some("0.00"));
    assert_eq!(zero.intrinsic_eur, None);
    assert!(!zero
        .warnings
        .iter()
        .any(|w| w.code == "missing_exchange_rate"));
}

#[test]
fn confirmed_zero_stock_keeps_euro_zero_with_a_foreign_quote_without_fx() {
    let mut s = fixture();
    s.m.movements[0].quantity_change = "0".into();
    s.m.movements[0].cost_amount = Some("0".into());
    s.m.quotes[0].currency = "USD".into();
    let r = s.report("2024-03-31");
    assert_eq!(r.status, "complete");
    assert_eq!(r.currency, "EUR");
    assert_eq!(r.valuation_eur.as_deref(), Some("0.00"));
    assert_eq!(r.intrinsic_original.as_deref(), Some("0.00"));
    assert!(r.source_ids.contains(&"quote-r".into()));
    assert!(!r.warnings.iter().any(|w| w.code == "missing_exchange_rate"));

    s.m.movements.clear();
    let unknown = s.report("2024-03-31");
    assert_eq!(unknown.status, "partial");
    assert_eq!(unknown.quantity_remaining, None);
    assert_eq!(unknown.valuation_eur, None);
    assert_eq!(unknown.premium_eur, None);
    assert_eq!(unknown.discount_eur, None);
}

#[test]
fn observation_date_excludes_future_purchase_stock_and_its_cash_warning() {
    let mut s = fixture();
    s.m.movements.push(movement(
        "future-buy",
        "purchase",
        "10",
        "2025-01-01",
        Some("20000"),
    ));
    let r = s.report("2024-03-31");
    assert_eq!(r.quantity_remaining.as_deref(), Some("2"));
    assert_eq!(r.cost_remaining_eur.as_deref(), Some("2400.00"));
    assert!(!r.warnings.iter().any(|w| w.object_id == "future-buy"));
}
