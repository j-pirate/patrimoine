//! Independent phase-4 persistence oracles. Every fixture is local and synthetic.
use patrimoine_application::{RegistryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use serde_json::Value;
use std::{fs, path::Path};
use tempfile::TempDir;

fn fraction(num: &str, den: &str) -> Fraction {
    Fraction {
        num: num.into(),
        den: den.into(),
    }
}

fn person(vault: &mut Vault, name: &str) -> Entity {
    vault
        .save_entity(SaveEntityInput {
            id: None,
            expected_revision_id: None,
            reason: "Création synthétique indépendante".into(),
            kind: "person".into(),
            display_name: name.into(),
            first_name: Some(name.into()),
            last_name: None,
            birth_date: None,
            residence_country: Some("FR".into()),
            preferred_currency: "EUR".into(),
            activity: String::new(),
            notes: String::new(),
            archived: false,
        })
        .unwrap()
}

fn edit_person(row: &Entity, name: &str, country: &str) -> SaveEntityInput {
    SaveEntityInput {
        id: Some(row.id.clone()),
        expected_revision_id: Some(row.revision_id.clone()),
        reason: "Profil civil corrigé, fixture indépendante".into(),
        kind: row.kind.clone(),
        display_name: name.into(),
        first_name: Some(name.into()),
        last_name: row.last_name.clone(),
        birth_date: row.birth_date.clone(),
        residence_country: Some(country.into()),
        preferred_currency: row.preferred_currency.clone(),
        activity: row.activity.clone(),
        notes: row.notes.clone(),
        archived: row.archived,
    }
}

fn asset(vault: &mut Vault, name: &str) -> Asset {
    vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Actif synthétique indépendant".into(),
            name: name.into(),
            country: "FR".into(),
            category: "real_estate".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap()
}

fn owned_asset(vault: &mut Vault) -> (Entity, Asset) {
    let owner = person(vault, "Alice");
    vault
        .save_membership(SaveMembershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Périmètre synthétique".into(),
            person_id: owner.id.clone(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            relationship: "Famille".into(),
        })
        .unwrap();
    let property = asset(vault, "Appartement synthétique");
    vault
        .save_ownership(SaveOwnershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Détention synthétique".into(),
            asset_id: property.id.clone(),
            owner_entity_id: owner.id.clone(),
            legal_share: Some(fraction("1", "1")),
            economic_share: Some(fraction("1", "1")),
            valid_from: "2020-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    (owner, property)
}

fn valuation(
    vault: &mut Vault,
    property: &Asset,
    amount: &str,
    currency: &str,
    date: &str,
) -> Valuation {
    vault
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Valeur synthétique indépendante".into(),
            target_kind: "asset".into(),
            target_id: property.id.clone(),
            amount: amount.into(),
            currency: currency.into(),
            effective_on: date.into(),
            source: "Oracle local indépendant".into(),
        })
        .unwrap()
}

fn request(date: &str, knowledge_sequence: Option<u64>) -> ConsolidationRequest {
    ConsolidationRequest {
        context: "family".into(),
        person_id: None,
        date: date.into(),
        knowledge_sequence,
    }
}

fn snapshot(vault: &Vault) -> Value {
    serde_json::to_value(vault.list_registry().unwrap()).unwrap()
}

fn counters(vault: &Vault) -> Vec<i64> {
    let status = vault.status().unwrap();
    let path = Path::new(&status.data_dir)
        .join("generations")
        .join(status.generation_id)
        .join("patrimoine.sqlite3");
    let connection = Connection::open(path).unwrap();
    ["objects", "revisions", "change_sets", "audit_events"]
        .into_iter()
        .map(|table| {
            connection
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap()
        })
        .collect()
}

fn assert_no_mutation(vault: &Vault, before: &Value, counts: &[i64], sequence: u64) {
    assert_eq!(&snapshot(vault), before);
    assert_eq!(counters(vault), counts);
    assert_eq!(vault.status().unwrap().mutation_sequence, sequence);
}

fn rate_input(base: &str, quote: &str, value: &str, date: &str) -> SaveExchangeRateInput {
    SaveExchangeRateInput {
        id: None,
        expected_revision_id: None,
        reason: "Cours synthétique indépendant".into(),
        base_currency: base.into(),
        quote_currency: quote.into(),
        rate: value.into(),
        effective_on: date.into(),
        source: Some("Oracle local indépendant".into()),
        notes: "Aucune source réseau".into(),
    }
}

fn rate(vault: &mut Vault, base: &str, quote: &str, value: &str, date: &str) -> ExchangeRate {
    vault
        .save_exchange_rate(rate_input(base, quote, value, date))
        .unwrap()
}

fn conversion(
    vault: &Vault,
    amount: &str,
    from: &str,
    to: &str,
    date: &str,
    k: Option<u64>,
) -> FxResult {
    vault
        .convert_currency(FxQuery {
            amount: amount.into(),
            from_currency: from.into(),
            to_currency: to.into(),
            date: date.into(),
            knowledge_sequence: k,
        })
        .unwrap()
}

fn period(
    kind: &str,
    target: &str,
    revision: &str,
    from: &str,
    to: Option<&str>,
) -> SaveProfilePeriodInput {
    SaveProfilePeriodInput {
        id: None,
        expected_revision_id: None,
        reason: "Période civile synthétique".into(),
        target_kind: kind.into(),
        target_id: target.into(),
        profile_revision_id: revision.into(),
        valid_from: from.into(),
        valid_to: to.map(str::to_owned),
    }
}

fn variation(vault: &Vault, from: &str, to: &str) -> VariationReport {
    vault
        .explain_variation(VariationRequest {
            context: "family".into(),
            person_id: None,
            from_date: from.into(),
            to_date: to.into(),
            knowledge_sequence: None,
        })
        .unwrap()
}

#[test]
fn dated_direct_quotes_never_use_the_next_observation() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let june = rate(&mut vault, "TND", "EUR", "0.30", "2026-06-01");
    rate(&mut vault, "TND", "EUR", "0.31", "2026-07-01");
    let result = conversion(&vault, "1000", "TND", "EUR", "2026-06-30", None);
    assert_eq!(result.exact_rate, Some(fraction("3", "10")));
    assert_eq!(result.exact_converted, Some(fraction("300", "1")));
    assert_eq!(result.amount_converted.as_deref(), Some("300.00"));
    assert_eq!(result.legs[0].revision_id, june.revision_id);
    assert_eq!(result.oldest_rate_date.as_deref(), Some("2026-06-01"));
    assert_eq!(result.age_days, Some(29));
}

#[test]
fn inverse_and_identity_keep_exact_values_and_tnd_minor_units() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let quote = rate(&mut vault, "TND", "EUR", "0.3", "2026-01-01");
    let result = conversion(&vault, "1", "EUR", "TND", "2026-01-01", None);
    assert_eq!(result.method, "inverse");
    assert_eq!(result.exact_converted, Some(fraction("10", "3")));
    assert_eq!(result.amount_converted.as_deref(), Some("3.333"));
    assert_eq!(result.legs[0].rate_id, quote.id);
    assert!(result.legs[0].inverted);
    let identity = conversion(&vault, "10.001", "TND", "TND", "1900-01-01", None);
    assert_eq!(identity.method, "identity");
    assert_eq!(identity.exact_rate, Some(fraction("1", "1")));
    assert_eq!(identity.exact_converted, Some(fraction("10001", "1000")));
    assert_eq!(identity.amount_converted.as_deref(), Some("10.001"));
    assert!(identity.legs.is_empty());
    let french_input = conversion(&vault, "1,2500", "EUR", "TND", "2026-01-01", None);
    assert_eq!(french_input.amount_original, "1.25");
    assert_eq!(french_input.exact_converted, Some(fraction("25", "6")));
    assert_eq!(french_input.amount_converted.as_deref(), Some("4.167"));
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    assert_eq!(
        vault
            .convert_currency(FxQuery {
                amount: " 1 000 ".into(),
                from_currency: "EUR".into(),
                to_currency: "TND".into(),
                date: "2026-01-01".into(),
                knowledge_sequence: None,
            })
            .unwrap_err()
            .code,
        "validation"
    );
    assert_no_mutation(&vault, &before, &counts, sequence);
}

#[test]
fn cross_via_eur_keeps_both_sources_and_the_oldest_date() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let tnd = rate(&mut vault, "TND", "EUR", "0.3", "2026-01-01");
    let chf = rate(&mut vault, "CHF", "EUR", "1.04", "2026-02-01");
    let result = conversion(&vault, "1", "TND", "CHF", "2026-02-02", None);
    assert_eq!(result.method, "cross");
    assert_eq!(result.exact_rate, Some(fraction("15", "52")));
    assert_eq!(result.exact_converted, Some(fraction("15", "52")));
    assert_eq!(result.amount_converted.as_deref(), Some("0.29"));
    assert_eq!(result.legs.len(), 2);
    assert_eq!(result.legs[0].revision_id, tnd.revision_id);
    assert_eq!(result.legs[1].revision_id, chf.revision_id);
    assert_eq!(
        result.legs[1].source.as_deref(),
        Some("Oracle local indépendant")
    );
    assert!(result.legs[1].inverted);
    assert_eq!(result.oldest_rate_date.as_deref(), Some("2026-01-01"));
    assert_eq!(result.age_days, Some(32));
    assert!(result
        .warnings
        .iter()
        .any(|w| w.code == "stale_exchange_rate"));
}

#[test]
fn secondary_usd_pivot_is_explicit_and_aed_has_no_assumed_parity() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    rate(&mut vault, "USD", "EUR", "0.8", "2026-01-01");
    let missing = conversion(&vault, "100", "AED", "EUR", "2026-01-02", None);
    assert_eq!(missing.status, "non_calculable");
    assert!(missing.exact_converted.is_none());
    rate(&mut vault, "AED", "USD", "0.25", "2026-01-02");
    let result = conversion(&vault, "100", "AED", "EUR", "2026-01-02", None);
    assert_eq!(result.method, "cross");
    assert_eq!(result.exact_rate, Some(fraction("1", "5")));
    assert_eq!(result.amount_converted.as_deref(), Some("20.00"));
    assert_eq!(result.legs[0].to_currency, "USD");
    assert_eq!(result.legs[1].to_currency, "EUR");
}

#[test]
fn a_future_only_quote_leaves_conversion_and_consolidation_unavailable() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let (_, property) = owned_asset(&mut vault);
    valuation(&mut vault, &property, "100", "CHF", "2026-01-01");
    rate(&mut vault, "CHF", "EUR", "1.05", "2026-02-01");
    let result = conversion(&vault, "100", "CHF", "EUR", "2026-01-31", None);
    assert_eq!(result.status, "non_calculable");
    assert_eq!(result.method, "missing");
    assert!(result.amount_converted.is_none());
    assert!(result.legs.is_empty());
    let report = vault
        .get_consolidation(request("2026-01-31", None))
        .unwrap();
    // No recorded contribution can be calculated; `partial` requires one.
    assert_eq!(report.status, "non_calculable");
    assert!(report
        .warnings
        .iter()
        .any(|w| w.code == "missing_exchange_rate"));
    assert!(report.contributions.is_empty());
}

#[test]
fn direct_quote_wins_and_divergent_inverse_remains_visible_as_a_warning() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let direct = rate(&mut vault, "CHF", "EUR", "1.05", "2026-01-01");
    rate(&mut vault, "EUR", "CHF", "1", "2026-01-01");
    let result = conversion(&vault, "100", "CHF", "EUR", "2026-01-01", None);
    assert_eq!(result.method, "direct");
    assert_eq!(result.amount_converted.as_deref(), Some("105.00"));
    assert_eq!(result.legs[0].rate_id, direct.id);
    assert!(result
        .warnings
        .iter()
        .any(|w| w.code == "inconsistent_exchange_rates"));
}

#[test]
fn stock_conversion_uses_report_date_and_preserves_original_precision() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let (_, property) = owned_asset(&mut vault);
    let value = valuation(&mut vault, &property, "10.001", "TND", "2026-01-01");
    rate(&mut vault, "TND", "EUR", "0.3", "2026-01-01");
    let newer = rate(&mut vault, "TND", "EUR", "0.4", "2026-06-01");
    let report = vault
        .get_consolidation(request("2026-06-30", None))
        .unwrap();
    assert_eq!(report.exact_gross, fraction("10001", "2500"));
    assert_eq!(report.gross_eur, "4.00");
    assert_eq!(report.contributions.len(), 1);
    let line = &report.contributions[0];
    assert_eq!(line.amount_original, "10.001");
    assert_eq!(line.currency, "TND");
    assert_eq!(
        line.valuation_revision_id.as_deref(),
        Some(value.revision_id.as_str())
    );
    assert_eq!(line.valuation_date.as_deref(), Some("2026-01-01"));
    assert!(line.source_ids.contains(&newer.revision_id));
    assert_eq!(
        vault.list_registry().unwrap().valuations[0].amount,
        "10.001"
    );
    assert_eq!(vault.list_registry().unwrap().valuations[0].currency, "TND");
}

#[test]
fn rate_and_valuation_corrections_reproduce_each_knowledge_sequence() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let (_, property) = owned_asset(&mut vault);
    let first_value = valuation(&mut vault, &property, "100", "CHF", "2026-01-01");
    let first_rate = rate(&mut vault, "CHF", "EUR", "1.05", "2026-01-01");
    let original_k = vault.status().unwrap().mutation_sequence;
    vault
        .save_valuation(SaveValuationInput {
            id: Some(first_value.id.clone()),
            expected_revision_id: Some(first_value.revision_id.clone()),
            reason: "Valeur corrigée rétroactivement".into(),
            target_kind: first_value.target_kind.clone(),
            target_id: first_value.target_id.clone(),
            amount: "200".into(),
            currency: "CHF".into(),
            effective_on: first_value.effective_on.clone(),
            source: "Justificatif synthétique de correction".into(),
        })
        .unwrap();
    let valuation_k = vault.status().unwrap().mutation_sequence;
    let mut corrected = rate_input("CHF", "EUR", "1.06", "2026-01-01");
    corrected.id = Some(first_rate.id.clone());
    corrected.expected_revision_id = Some(first_rate.revision_id.clone());
    corrected.reason = "Cours corrigé rétroactivement".into();
    let final_rate = vault.save_exchange_rate(corrected).unwrap();
    assert_eq!(first_rate.id, final_rate.id);
    assert_ne!(first_rate.revision_id, final_rate.revision_id);
    for (k, expected) in [
        (Some(original_k), "105.00"),
        (Some(valuation_k), "210.00"),
        (None, "212.00"),
    ] {
        let report = vault.get_consolidation(request("2026-01-31", k)).unwrap();
        assert_eq!(report.gross_eur, expected);
        assert_eq!(
            report.knowledge_sequence,
            k.unwrap_or(vault.status().unwrap().mutation_sequence)
        );
    }
    let old = vault.list_registry_at(Some(original_k)).unwrap();
    assert_eq!(old.valuations[0].revision_id, first_value.revision_id);
    assert_eq!(old.exchange_rates[0].revision_id, first_rate.revision_id);
    let history = vault.list_registry_history().unwrap();
    assert_eq!(
        history.latest_sequence,
        vault.status().unwrap().mutation_sequence
    );
    let corrections: Vec<_> = history
        .events
        .iter()
        .filter(|e| e.object_id == first_rate.id)
        .collect();
    assert_eq!(corrections.len(), 2);
    assert!(corrections
        .iter()
        .any(|e| e.revision_id == first_rate.revision_id));
    assert!(corrections
        .iter()
        .any(|e| e.revision_id == final_rate.revision_id
            && e.reason == "Cours corrigé rétroactivement"));
}

#[test]
fn voided_rate_remains_available_only_before_its_void_sequence() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let quote = rate(&mut vault, "CHF", "EUR", "1.05", "2026-01-01");
    let k = vault.status().unwrap().mutation_sequence;
    vault
        .void_registry_record(VoidRecordInput {
            id: quote.id.clone(),
            expected_revision_id: quote.revision_id,
            reason: "Cours synthétique annulé".into(),
        })
        .unwrap();
    assert_eq!(
        conversion(&vault, "100", "CHF", "EUR", "2026-01-01", Some(k))
            .amount_converted
            .as_deref(),
        Some("105.00")
    );
    assert!(conversion(&vault, "100", "CHF", "EUR", "2026-01-01", None)
        .amount_converted
        .is_none());
    assert!(vault.list_registry().unwrap().exchange_rates.is_empty());
    assert!(vault
        .list_registry_history()
        .unwrap()
        .events
        .iter()
        .any(|e| e.object_id == quote.id && e.is_void));
}

#[test]
fn invalid_rates_reject_atomically_and_eighteen_decimals_are_not_truncated() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let exact = rate(
        &mut vault,
        "CHF",
        "EUR",
        "0.123456789012345678",
        "2026-01-01",
    );
    assert_eq!(exact.rate, "0.123456789012345678");
    assert_eq!(
        conversion(&vault, "1", "CHF", "EUR", "2026-01-01", None).exact_converted,
        Some(fraction("61728394506172839", "500000000000000000"))
    );
    let maximum = "999999999999999999999999999999999999";
    assert_eq!(
        rate(&mut vault, "USD", "EUR", maximum, "2026-01-01").rate,
        maximum
    );
    assert_eq!(
        conversion(&vault, "1", "USD", "EUR", "2026-01-01", None).exact_rate,
        Some(fraction(maximum, "1"))
    );
    let mixed_bound = "123456789012345678.123456789012345678";
    assert_eq!(
        rate(&mut vault, "USD", "EUR", mixed_bound, "2026-02-01").rate,
        mixed_bound
    );
    assert_eq!(
        rate(&mut vault, "CHF", "EUR", " 1,0500 ", "2026-02-01").rate,
        "1.05"
    );
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    for invalid in [
        "0",
        "-0.1",
        "NaN",
        "Infinity",
        "1e-3",
        "0.1234567890123456789",
        "1234567890123456789012345678901234567",
    ] {
        assert!(
            vault
                .save_exchange_rate(rate_input("USD", "EUR", invalid, "2026-01-01"))
                .is_err(),
            "invalid rate accepted: {invalid}"
        );
        assert_no_mutation(&vault, &before, &counts, sequence);
    }
}

#[test]
fn duplicate_pair_observations_and_stale_corrections_leave_no_audit_trace() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let first = rate(&mut vault, "CHF", "EUR", "1.05", "2026-01-01");
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    assert!(vault
        .save_exchange_rate(rate_input("CHF", "EUR", "1.06", "2026-01-01"))
        .is_err());
    assert_no_mutation(&vault, &before, &counts, sequence);
    let mut correction = rate_input("CHF", "EUR", "1.06", "2026-01-01");
    correction.id = Some(first.id.clone());
    correction.expected_revision_id = Some(first.revision_id);
    vault.save_exchange_rate(correction.clone()).unwrap();
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    assert_eq!(
        vault.save_exchange_rate(correction).unwrap_err().code,
        "conflict"
    );
    assert_no_mutation(&vault, &before, &counts, sequence);
}

#[test]
fn civil_entity_profiles_reference_old_revisions_without_leaking_future_knowledge() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let owner = person(&mut vault, "Alice");
    vault
        .save_membership(SaveMembershipInput {
            id: None,
            expected_revision_id: None,
            reason: "Périmètre synthétique".into(),
            person_id: owner.id.clone(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            relationship: "Famille".into(),
        })
        .unwrap();
    let mut company_input = edit_person(&owner, "Société avant", "FR");
    company_input.id = None;
    company_input.expected_revision_id = None;
    company_input.kind = "company".into();
    company_input.first_name = None;
    let old = vault.save_entity(company_input).unwrap();
    vault
        .save_participation(SaveParticipationInput {
            id: None,
            expected_revision_id: None,
            reason: "Participation synthétique".into(),
            owner_entity_id: owner.id,
            company_id: old.id.clone(),
            legal_share: Some(fraction("1", "1")),
            economic_share: Some(fraction("1", "1")),
            voting_share: None,
            valid_from: "2020-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    vault
        .save_company_policy(SaveCompanyPolicyInput {
            id: None,
            expected_revision_id: None,
            reason: "Valeur globale synthétique".into(),
            company_id: old.id.clone(),
            mode: "global_equity".into(),
            coverage_complete: true,
            liability_qualified: true,
            valid_from: "2020-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    vault
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Capitaux propres synthétiques".into(),
            target_kind: "company".into(),
            target_id: old.id.clone(),
            amount: "250".into(),
            currency: "EUR".into(),
            effective_on: "2020-01-01".into(),
            source: "Capitaux propres, périmètre complet".into(),
        })
        .unwrap();
    vault
        .save_profile_period(period(
            "entity",
            &old.id,
            &old.revision_id,
            "2020-01-01",
            Some("2026-01-01"),
        ))
        .unwrap();
    let original_k = vault.status().unwrap().mutation_sequence;
    let mut input = edit_person(&old, "Société après", "CH");
    input.first_name = None;
    let new = vault.save_entity(input).unwrap();
    vault
        .save_profile_period(period(
            "entity",
            &new.id,
            &new.revision_id,
            "2026-01-01",
            None,
        ))
        .unwrap();
    let historical = vault.list_registry_at(Some(original_k)).unwrap();
    assert!(!historical
        .profile_revisions
        .iter()
        .any(|r| r.revision_id == new.revision_id));
    assert_eq!(
        historical
            .entities
            .iter()
            .find(|e| e.id == old.id)
            .unwrap()
            .display_name,
        "Société avant"
    );
    let historical_report = vault
        .get_consolidation(request("2026-01-01", Some(original_k)))
        .unwrap();
    assert_eq!(historical_report.gross_eur, "250.00");
    assert_eq!(historical_report.contributions[0].label, "Société avant");
    assert!(historical_report.contributions[0].country.is_none());
    assert!(historical_report
        .warnings
        .iter()
        .any(|warning| warning.code == "undated_profile"));
    for (date, name, country) in [
        ("2025-12-31", "Société avant", "FR"),
        ("2026-01-01", "Société après", "CH"),
    ] {
        let report = vault.get_consolidation(request(date, None)).unwrap();
        assert_eq!(report.gross_eur, "250.00");
        assert_eq!(report.contributions[0].label, name);
        assert_eq!(report.contributions[0].country.as_deref(), Some(country));
    }
}

#[test]
fn civil_asset_profiles_apply_exclusive_boundaries_without_changing_rights() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let (_, old) = owned_asset(&mut vault);
    valuation(&mut vault, &old, "400", "EUR", "2020-01-01");
    vault
        .save_profile_period(period(
            "asset",
            &old.id,
            &old.revision_id,
            "2020-01-01",
            Some("2026-01-01"),
        ))
        .unwrap();
    let old_k = vault.status().unwrap().mutation_sequence;
    let new = vault
        .save_asset(SaveAssetInput {
            id: Some(old.id.clone()),
            expected_revision_id: Some(old.revision_id.clone()),
            reason: "Profil daté de l’actif".into(),
            name: "Appartement après".into(),
            country: "TN".into(),
            category: old.category.clone(),
            notes: old.notes.clone(),
            archived: false,
        })
        .unwrap();
    vault
        .save_profile_period(period(
            "asset",
            &new.id,
            &new.revision_id,
            "2026-01-01",
            None,
        ))
        .unwrap();
    let snapshot = vault.list_registry_at(Some(old_k)).unwrap();
    assert!(snapshot
        .asset_profile_revisions
        .iter()
        .any(|r| r.revision_id == old.revision_id));
    assert!(!snapshot
        .asset_profile_revisions
        .iter()
        .any(|r| r.revision_id == new.revision_id));
    for (date, name, country) in [
        ("2025-12-31", old.name.as_str(), "FR"),
        ("2026-01-01", new.name.as_str(), "TN"),
    ] {
        let report = vault.get_consolidation(request(date, None)).unwrap();
        assert_eq!(report.exact_gross, fraction("400", "1"));
        assert_eq!(report.contributions[0].label, name);
        assert_eq!(report.contributions[0].country.as_deref(), Some(country));
        assert_eq!(report.contributions[0].weight, fraction("1", "1"));
    }
}

#[test]
fn invalid_profile_sources_overlaps_and_stale_edits_leave_no_trace() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let first = person(&mut vault, "Alice");
    let other = person(&mut vault, "Autre identité");
    let property = asset(&mut vault, "Actif indépendant");
    let removed = person(&mut vault, "Profil annulé");
    let removed_asset = asset(&mut vault, "Actif annulé");
    let removed_asset_profile = removed_asset.revision_id.clone();
    vault
        .void_registry_record(VoidRecordInput {
            id: removed.id.clone(),
            expected_revision_id: removed.revision_id,
            reason: "Annulation synthétique du profil".into(),
        })
        .unwrap();
    vault
        .void_registry_record(VoidRecordInput {
            id: removed_asset.id.clone(),
            expected_revision_id: removed_asset.revision_id,
            reason: "Annulation synthétique de l’actif sans dépendance".into(),
        })
        .unwrap();
    let tombstone = vault
        .list_registry_history()
        .unwrap()
        .events
        .into_iter()
        .find(|event| event.object_id == removed.id && event.is_void)
        .unwrap()
        .revision_id;
    let saved = vault
        .save_profile_period(period(
            "entity",
            &first.id,
            &first.revision_id,
            "2020-01-01",
            Some("2026-01-01"),
        ))
        .unwrap();
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    let invalid = [
        period("entity", &first.id, &other.revision_id, "2026-01-01", None),
        period(
            "entity",
            &first.id,
            &property.revision_id,
            "2026-01-01",
            None,
        ),
        period(
            "asset",
            &property.id,
            &first.revision_id,
            "2026-01-01",
            None,
        ),
        period("entity", &removed.id, &tombstone, "2026-01-01", None),
        period("entity", &first.id, &first.revision_id, "2025-12-31", None),
        period(
            "entity",
            &first.id,
            &first.revision_id,
            "2026-01-01",
            Some("2026-01-01"),
        ),
    ];
    for input in invalid {
        assert!(vault.save_profile_period(input).is_err());
        assert_no_mutation(&vault, &before, &counts, sequence);
    }
    let mut correction = period(
        "entity",
        &first.id,
        &first.revision_id,
        "2020-01-01",
        Some("2025-01-01"),
    );
    correction.id = Some(saved.id);
    correction.expected_revision_id = Some(saved.revision_id);
    vault.save_profile_period(correction.clone()).unwrap();
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    assert_eq!(
        vault.save_profile_period(correction).unwrap_err().code,
        "conflict"
    );
    assert_no_mutation(&vault, &before, &counts, sequence);
    drop(vault);
    let reopened = Vault::open(temp.path().join("coffre")).unwrap();
    assert_eq!(snapshot(&reopened), before);
    let reopened = reopened.list_registry().unwrap();
    assert!(!reopened
        .entities
        .iter()
        .any(|entity| entity.id == removed.id));
    assert!(!reopened
        .assets
        .iter()
        .any(|asset| asset.id == removed_asset.id));
    assert!(reopened
        .asset_profile_revisions
        .iter()
        .any(|asset| asset.revision_id == removed_asset_profile));
}

#[test]
fn every_historical_query_rejects_unrecorded_future_knowledge() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    person(&mut vault, "Alice");
    rate(&mut vault, "CHF", "EUR", "1.05", "2026-01-01");
    let before = snapshot(&vault);
    let counts = counters(&vault);
    let sequence = vault.status().unwrap().mutation_sequence;
    let future = sequence + 1;
    assert_eq!(
        vault.list_registry_at(Some(future)).unwrap_err().code,
        "validation"
    );
    assert_eq!(
        vault
            .get_consolidation(request("2026-01-01", Some(future)))
            .unwrap_err()
            .code,
        "validation"
    );
    assert_eq!(
        vault
            .convert_currency(FxQuery {
                amount: "100".into(),
                from_currency: "CHF".into(),
                to_currency: "EUR".into(),
                date: "2026-01-01".into(),
                knowledge_sequence: Some(future),
            })
            .unwrap_err()
            .code,
        "validation"
    );
    assert_eq!(
        vault
            .explain_variation(VariationRequest {
                context: "family".into(),
                person_id: None,
                from_date: "2026-01-01".into(),
                to_date: "2026-02-01".into(),
                knowledge_sequence: Some(future),
            })
            .unwrap_err()
            .code,
        "validation"
    );
    assert_no_mutation(&vault, &before, &counts, sequence);
}

#[test]
fn persisted_variation_separates_price_and_fx_before_rounding() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let (_, property) = owned_asset(&mut vault);
    valuation(&mut vault, &property, "1000", "TND", "2026-01-01");
    valuation(&mut vault, &property, "1200", "TND", "2026-02-01");
    let old = rate(&mut vault, "TND", "EUR", "0.3", "2026-01-01");
    let new = rate(&mut vault, "TND", "EUR", "0.4", "2026-02-01");
    let result = variation(&vault, "2026-01-01", "2026-02-01");
    assert_eq!(result.opening.gross_eur, "300.00");
    assert_eq!(result.closing.gross_eur, "480.00");
    assert_eq!(result.exact_delta, fraction("180", "1"));
    assert_eq!(result.exact_explained, fraction("180", "1"));
    assert_eq!(result.exact_residual, Some(fraction("0", "1")));
    assert_eq!(result.status, "complete");
    assert_eq!(
        result
            .steps
            .iter()
            .map(|step| step.code.as_str())
            .collect::<Vec<_>>(),
        ["scope_method", "rights", "valuation", "fx"]
    );
    let price = result
        .steps
        .iter()
        .find(|step| step.code == "valuation")
        .unwrap();
    assert_eq!(price.exact_amount, Some(fraction("60", "1")));
    assert_eq!(price.dates.fx_date, "2026-01-01");
    let fx = result.steps.iter().find(|step| step.code == "fx").unwrap();
    assert_eq!(fx.exact_amount, Some(fraction("120", "1")));
    assert_eq!(fx.dates.valuation_date, "2026-02-01");
    assert!(fx.source_ids.contains(&old.revision_id));
    assert!(fx.source_ids.contains(&new.revision_id));
}

#[test]
fn persisted_variation_keeps_missing_opening_reference_unknown() {
    let temp = TempDir::new().unwrap();
    let mut vault = Vault::open(temp.path().join("coffre")).unwrap();
    let (_, property) = owned_asset(&mut vault);
    valuation(&mut vault, &property, "1000", "EUR", "2026-02-01");
    let result = variation(&vault, "2026-01-01", "2026-02-01");
    assert_eq!(result.opening.gross_eur, "0.00");
    assert_eq!(result.closing.gross_eur, "1000.00");
    assert_eq!(result.status, "partial");
    assert_eq!(result.exact_explained, fraction("0", "1"));
    assert_eq!(result.exact_residual, Some(fraction("1000", "1")));
    let price = result
        .steps
        .iter()
        .find(|step| step.code == "valuation")
        .unwrap();
    assert!(price.amount_eur.is_none());
    assert!(price.exact_amount.is_none());
    assert!(result
        .warnings
        .iter()
        .any(|warning| warning.code == "missing_valuation"));
}

#[test]
fn backup_restore_preserves_fx_profiles_history_and_document_bytes() {
    let temp = TempDir::new().unwrap();
    let mut source = Vault::open(temp.path().join("source")).unwrap();
    let (owner, property) = owned_asset(&mut source);
    valuation(&mut source, &property, "1234.567", "TND", "2026-01-01");
    let first = rate(
        &mut source,
        "TND",
        "EUR",
        "0.300000000000000001",
        "2026-01-01",
    );
    let historical_k = source.status().unwrap().mutation_sequence;
    let mut correction = rate_input("TND", "EUR", "0.31", "2026-01-01");
    correction.id = Some(first.id);
    correction.expected_revision_id = Some(first.revision_id);
    source.save_exchange_rate(correction).unwrap();
    source
        .save_profile_period(period(
            "entity",
            &owner.id,
            &owner.revision_id,
            "2020-01-01",
            None,
        ))
        .unwrap();
    source
        .save_profile_period(period(
            "asset",
            &property.id,
            &property.revision_id,
            "2020-01-01",
            None,
        ))
        .unwrap();
    let bytes = b"Original synthetique\n1234.567 TND\n";
    let original = temp.path().join("justificatif.txt");
    fs::write(&original, bytes).unwrap();
    let document = source
        .import_document(&original, "Justificatif du cours")
        .unwrap();
    let state = snapshot(&source);
    let historical =
        serde_json::to_value(source.list_registry_at(Some(historical_k)).unwrap()).unwrap();
    let history = serde_json::to_value(source.list_registry_history().unwrap()).unwrap();
    let current_report = serde_json::to_value(
        source
            .get_consolidation(request("2026-01-31", None))
            .unwrap(),
    )
    .unwrap();
    let old_report = serde_json::to_value(
        source
            .get_consolidation(request("2026-01-31", Some(historical_k)))
            .unwrap(),
    )
    .unwrap();
    let current_fx = serde_json::to_value(conversion(
        &source,
        "1234.567",
        "TND",
        "EUR",
        "2026-01-31",
        None,
    ))
    .unwrap();
    let backup = temp.path().join("phase4.zip");
    source.create_backup(&backup).unwrap();
    let archive_bytes = fs::read(&backup).unwrap();
    let target_root = temp.path().join("cible");
    let mut target = Vault::open(&target_root).unwrap();
    let old_generation = target.status().unwrap().generation_id;
    let inspection = target.inspect_restore(&backup).unwrap();
    assert_eq!(inspection.schema_version, 6);
    let status = target
        .activate_restore(
            &inspection.token,
            true,
            Some(&temp.path().join("securite.zip")),
        )
        .unwrap();
    assert_eq!(
        status.mutation_sequence,
        state["knowledge_sequence"].as_u64().unwrap()
    );
    assert!(target_root
        .join("generations")
        .join(old_generation)
        .join("patrimoine.sqlite3")
        .is_file());
    drop(target);
    let target = Vault::open(&target_root).unwrap();
    assert_eq!(snapshot(&target), state);
    assert_eq!(
        serde_json::to_value(target.list_registry_at(Some(0)).unwrap()).unwrap(),
        serde_json::to_value(RegistrySnapshot::default()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(target.list_registry_at(Some(historical_k)).unwrap()).unwrap(),
        historical
    );
    assert_eq!(
        serde_json::to_value(target.list_registry_history().unwrap()).unwrap(),
        history
    );
    assert_eq!(
        serde_json::to_value(
            target
                .get_consolidation(request("2026-01-31", None))
                .unwrap()
        )
        .unwrap(),
        current_report
    );
    assert_eq!(
        serde_json::to_value(
            target
                .get_consolidation(request("2026-01-31", Some(historical_k)))
                .unwrap()
        )
        .unwrap(),
        old_report
    );
    assert_eq!(
        serde_json::to_value(conversion(
            &target,
            "1234.567",
            "TND",
            "EUR",
            "2026-01-31",
            None
        ))
        .unwrap(),
        current_fx
    );
    let exported = temp.path().join("restaure.txt");
    target.export_document(&document.id, &exported).unwrap();
    assert_eq!(fs::read(exported).unwrap(), bytes);
    assert_eq!(fs::read(backup).unwrap(), archive_bytes);
}

#[test]
fn knowledge_zero_remains_empty_after_corrections_and_does_not_mutate_the_vault() {
    let temp = TempDir::new().unwrap();
    let root = temp.path().join("coffre");
    let mut vault = Vault::open(&root).unwrap();
    let owner = person(&mut vault, "Alice");
    vault
        .save_entity(edit_person(&owner, "Alice corrigée", "TN"))
        .unwrap();
    rate(&mut vault, "USD", "EUR", "0.9", "2026-01-01");
    let baseline = snapshot(&vault);
    let before = vault.status().unwrap().mutation_sequence;
    let at_zero = vault.list_registry_at(Some(0)).unwrap();
    assert_eq!(
        serde_json::to_value(&at_zero).unwrap(),
        serde_json::to_value(RegistrySnapshot::default()).unwrap()
    );
    let fx = conversion(&vault, "100", "USD", "EUR", "2026-01-01", Some(0));
    assert_eq!(fx.status, "non_calculable");
    assert!(fx.legs.is_empty());
    assert_eq!(fx.knowledge_sequence, 0);
    assert_eq!(vault.status().unwrap().mutation_sequence, before);
    assert_eq!(snapshot(&vault), baseline);
    drop(vault);
    let reopened = Vault::open(root).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.list_registry_at(Some(0)).unwrap()).unwrap(),
        serde_json::to_value(RegistrySnapshot::default()).unwrap()
    );
    assert_eq!(reopened.status().unwrap().mutation_sequence, before);
}
