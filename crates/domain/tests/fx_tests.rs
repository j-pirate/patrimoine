use patrimoine_domain::{exact::*, *};

fn fraction(n: &str, d: &str) -> Fraction {
    Fraction {
        num: n.into(),
        den: d.into(),
    }
}

fn query(amount: &str, from: &str, to: &str, date: &str) -> FxQuery {
    FxQuery {
        amount: amount.into(),
        from_currency: from.into(),
        to_currency: to.into(),
        date: date.into(),
        knowledge_sequence: None,
    }
}

fn rate(s: &mut RegistrySnapshot, id: &str, from: &str, to: &str, value: &str, date: &str) {
    s.exchange_rates.push(ExchangeRate {
        id: id.into(),
        revision_id: format!("{id}-r"),
        base_currency: from.into(),
        quote_currency: to.into(),
        rate: value.into(),
        effective_on: date.into(),
        source: Some(format!("Source {id}")),
        notes: String::new(),
    });
}

#[test]
fn rate_sources_have_independent_bounded_exact_precision() {
    assert_eq!(
        fraction_of(&parse_rate("0.000000000000000001").unwrap()),
        fraction("1", "1000000000000000000")
    );
    assert!(parse_rate("999999999999999999999999999999999999").is_ok());
    assert!(parse_rate("123456789012345678.123456789012345678").is_ok());
    for invalid in [
        "0",
        "-1",
        "1e3",
        "NaN",
        "1.00",
        "1,2",
        "01",
        "0.0000000000000000001",
        "1000000000000000000000000000000000000",
    ] {
        assert!(parse_rate(invalid).is_err(), "accepted {invalid}");
    }
    assert_eq!(normalize_rate_input(" 1,2500 ").unwrap(), "1.25");
    assert!(normalize_rate_input("1,2.3").is_err());
    assert!(normalize_rate_input(&"1".repeat(100_000)).is_err());
}

#[test]
fn latest_direct_rate_applies_to_report_date_without_future_leakage() {
    let mut s = RegistrySnapshot {
        knowledge_sequence: 17,
        ..Default::default()
    };
    rate(&mut s, "old", "TND", "EUR", "0.3", "2024-01-01");
    rate(&mut s, "current", "TND", "EUR", "0.31", "2024-02-01");
    rate(&mut s, "future", "TND", "EUR", "0.9", "2024-03-01");
    let r = convert_currency(&s, &query("1000", "TND", "EUR", "2024-02-29")).unwrap();
    assert_eq!(r.method, "direct");
    assert_eq!(r.amount_converted.as_deref(), Some("310.00"));
    assert_eq!(r.exact_converted, Some(fraction("310", "1")));
    assert_eq!(r.knowledge_sequence, 17);
    assert_eq!(r.legs[0].rate_id, "current");
    assert_eq!(r.legs[0].revision_id, "current-r");
    assert_eq!(r.legs[0].source.as_deref(), Some("Source current"));
    assert_eq!(r.amount_original, "1000");
}

#[test]
fn identity_is_exact_and_requires_no_manual_rate() {
    let r = convert_currency(
        &RegistrySnapshot::default(),
        &query("1.005", "EUR", "EUR", "2024-01-01"),
    )
    .unwrap();
    assert_eq!(r.method, "identity");
    assert_eq!(r.exact_rate, Some(fraction("1", "1")));
    assert_eq!(r.amount_converted.as_deref(), Some("1.00"));
    assert!(r.legs.is_empty());
    assert_eq!(r.age_days, None);
    assert!(r.warnings.is_empty());
}

#[test]
fn inverse_preserves_thirds_and_tnd_minor_units_until_final_rounding() {
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "inverse", "TND", "EUR", "3", "2024-01-01");
    let r = convert_currency(&s, &query("1", "EUR", "TND", "2024-01-01")).unwrap();
    assert_eq!(r.method, "inverse");
    assert_eq!(r.exact_converted, Some(fraction("1", "3")));
    assert_eq!(r.amount_converted.as_deref(), Some("0.333"));
    assert!(r.legs[0].inverted);
    assert_eq!(r.legs[0].rate, "3");
    assert_eq!(r.legs[0].exact_rate, fraction("1", "3"));
}

#[test]
fn cross_rate_carries_both_dated_sources_and_exact_reciprocal() {
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "tnd", "TND", "EUR", "0.3", "2024-01-01");
    rate(&mut s, "chf", "CHF", "EUR", "1.04", "2024-01-10");
    let r = convert_currency(&s, &query("1", "TND", "CHF", "2024-01-31")).unwrap();
    assert_eq!(r.method, "cross");
    assert_eq!(r.exact_rate, Some(fraction("15", "52")));
    assert_eq!(r.amount_converted.as_deref(), Some("0.29"));
    assert_eq!(r.legs.len(), 2);
    assert_eq!(r.legs[0].rate_id, "tnd");
    assert_eq!(r.legs[1].rate_id, "chf");
    assert!(r.legs[1].inverted);
    assert_eq!(r.oldest_rate_date.as_deref(), Some("2024-01-01"));
    assert_eq!(r.age_days, Some(30));
}

#[test]
fn contradictory_paths_warn_without_averaging_or_overriding_direct_priority() {
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "direct", "TND", "CHF", "0.4", "2024-01-01");
    rate(&mut s, "reverse", "CHF", "TND", "2", "2024-01-01");
    rate(&mut s, "a", "TND", "EUR", "0.3", "2024-01-01");
    rate(&mut s, "b", "EUR", "CHF", "1", "2024-01-01");
    let r = convert_currency(&s, &query("100", "TND", "CHF", "2024-01-01")).unwrap();
    assert_eq!(r.method, "direct");
    assert_eq!(r.amount_converted.as_deref(), Some("40.00"));
    assert!(r
        .warnings
        .iter()
        .any(|w| w.code == "inconsistent_exchange_rates" && w.message.contains("reverse")));
    s.exchange_rates.remove(0);
    let r = convert_currency(&s, &query("100", "TND", "CHF", "2024-01-01")).unwrap();
    assert_eq!(r.method, "inverse");
    assert_eq!(r.amount_converted.as_deref(), Some("50.00"));
}

#[test]
fn pivot_priority_is_stable_under_storage_order_and_does_not_use_three_legs() {
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "usd-a", "TND", "USD", "2", "2024-01-01");
    rate(&mut s, "usd-b", "USD", "CHF", "3", "2024-01-01");
    rate(&mut s, "eur-a", "TND", "EUR", "0.3", "2024-01-01");
    rate(&mut s, "eur-b", "EUR", "CHF", "1", "2024-01-01");
    for _ in 0..2 {
        let r = convert_currency(&s, &query("10", "TND", "CHF", "2024-01-01")).unwrap();
        assert_eq!(r.amount_converted.as_deref(), Some("3.00"));
        assert_eq!(r.legs[0].rate_id, "eur-a");
        s.exchange_rates.reverse();
    }
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "a", "TND", "USD", "2", "2024-01-01");
    rate(&mut s, "b", "USD", "CHF", "3", "2024-01-01");
    rate(&mut s, "c", "CHF", "AED", "4", "2024-01-01");
    assert_eq!(
        convert_currency(&s, &query("1", "TND", "AED", "2024-01-01"))
            .unwrap()
            .status,
        "non_calculable"
    );
}

#[test]
fn missing_and_future_rates_never_become_zero_or_assumed_pegs() {
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "future", "AED", "USD", "0.27", "2024-02-01");
    let r = convert_currency(&s, &query("0", "AED", "USD", "2024-01-01")).unwrap();
    assert_eq!(r.status, "non_calculable");
    assert_eq!(r.method, "missing");
    assert_eq!(r.amount_converted, None);
    assert_eq!(r.exact_converted, None);
    assert!(r.warnings.iter().any(|w| w.code == "missing_exchange_rate"));
}

#[test]
fn stale_threshold_uses_civil_days_including_leap_day_and_keeps_identified_amount() {
    let mut s = RegistrySnapshot::default();
    rate(&mut s, "r", "TND", "EUR", "0.3", "2024-02-01");
    let fresh = convert_currency(&s, &query("100", "TND", "EUR", "2024-03-02")).unwrap();
    assert_eq!(fresh.age_days, Some(30));
    assert!(!fresh
        .warnings
        .iter()
        .any(|w| w.code == "stale_exchange_rate"));
    let stale = convert_currency(&s, &query("100", "TND", "EUR", "2024-03-03")).unwrap();
    assert_eq!(stale.age_days, Some(31));
    assert_eq!(stale.amount_converted.as_deref(), Some("30.00"));
    assert!(stale
        .warnings
        .iter()
        .any(|w| w.code == "stale_exchange_rate"));
}

#[test]
fn supported_precision_is_explicit_and_half_even_is_symmetric() {
    for (source, code, expected) in [
        ("1.0005", "TND", "1.000"),
        ("1.0015", "TND", "1.002"),
        ("-0.0005", "TND", "0.000"),
        ("-1.015", "USD", "-1.02"),
    ] {
        assert_eq!(
            format_currency(&parse_decimal(source).unwrap(), code).unwrap(),
            expected
        );
    }
    assert!(format_currency(&parse_decimal("1").unwrap(), "ZZZ").is_err());
    assert!(convert_currency(
        &RegistrySnapshot::default(),
        &query("1", "ZZZ", "ZZZ", "2024-01-01")
    )
    .is_err());
}

#[test]
fn query_boundaries_and_incoherent_sources_are_rejected() {
    let s = RegistrySnapshot::default();
    let mut q = query("1", "EUR", "EUR", "2024-01-01");
    q.knowledge_sequence = Some(1);
    assert!(convert_currency(&s, &q).is_err());
    for q in [
        query("1", "EUR", "EUR", "2023-02-29"),
        query("1.00", "EUR", "EUR", "2024-01-01"),
        query("1", "eur", "EUR", "2024-01-01"),
    ] {
        assert!(convert_currency(&s, &q).is_err());
    }
    let mut s = s;
    rate(&mut s, "a", "TND", "EUR", "0.3", "2024-01-01");
    rate(&mut s, "b", "TND", "EUR", "0.4", "2024-01-01");
    assert!(convert_currency(&s, &query("1", "TND", "EUR", "2024-01-01")).is_err());
}
