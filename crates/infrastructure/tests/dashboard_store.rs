//! SQLite-backed dashboard checks with independent monetary expectations.
use patrimoine_application::{
    CompanyService, DashboardService, DebtService, FinancialService, RegistryService,
};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
};
use tempfile::TempDir;

fn fraction(num: &str, den: &str) -> Fraction {
    Fraction {
        num: num.into(),
        den: den.into(),
    }
}

fn request() -> DashboardRequest {
    DashboardRequest {
        context: "family".into(),
        entity_id: None,
        date: "2025-03-31".into(),
        from_date: "2025-01-01".into(),
        interval: "monthly".into(),
        knowledge_sequence: None,
        filters: DashboardFilters::default(),
    }
}

fn entity(v: &mut Vault, name: &str, kind: &str) -> Entity {
    v.save_entity(SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Identité du tableau de bord synthétique".into(),
        kind: kind.into(),
        display_name: name.into(),
        first_name: None,
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

fn member(v: &mut Vault, e: &Entity) {
    v.save_membership(SaveMembershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Périmètre familial explicite".into(),
        person_id: e.id.clone(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        relationship: "Famille synthétique".into(),
    })
    .unwrap();
}

fn owned_asset(
    v: &mut Vault,
    e: &Entity,
    name: &str,
    country: &str,
    amount: &str,
) -> (Asset, Valuation) {
    let a = v
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Actif du tableau de bord synthétique".into(),
            name: name.into(),
            country: country.into(),
            category: "immobilier".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    v.save_ownership(SaveOwnershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Droit économique explicite".into(),
        asset_id: a.id.clone(),
        owner_entity_id: e.id.clone(),
        legal_share: Some(fraction("1", "1")),
        economic_share: Some(fraction("1", "1")),
        valid_from: "2024-01-01".into(),
        valid_to: None,
    })
    .unwrap();
    let value = v
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Estimation synthétique sourcée".into(),
            target_kind: "asset".into(),
            target_id: a.id.clone(),
            amount: amount.into(),
            currency: "EUR".into(),
            effective_on: "2025-01-01".into(),
            source: "Source synthétique dashboard".into(),
        })
        .unwrap();
    (a, value)
}

fn coverage(v: &mut Vault) {
    v.save_finance_coverage(SaveFinanceCoverageInput {
        id: None,
        expected_revision_id: None,
        reason: "Inventaire synthétique fermé".into(),
        valid_from: "2024-01-01".into(),
        valid_to: None,
        assets_complete: true,
        debts_complete: true,
        rentals_complete: true,
        notes: "Deux actifs et un financement interne explicites.".into(),
    })
    .unwrap();
}

struct Fixture {
    temporary: TempDir,
    vault: Vault,
    alice: Entity,
    bob: Entity,
    alice_asset: Asset,
    alice_value: Valuation,
    debt: DebtContract,
}

fn fixture() -> Fixture {
    fixture_with_profiles(true)
}

fn fixture_with_profiles(dated: bool) -> Fixture {
    let temporary = TempDir::new().unwrap();
    let mut vault = Vault::open(temporary.path().join("vault")).unwrap();
    let alice = entity(&mut vault, "Alice synthétique", "person");
    let bob = entity(&mut vault, "Benoît synthétique", "person");
    member(&mut vault, &alice);
    member(&mut vault, &bob);
    let (alice_asset, alice_value) =
        owned_asset(&mut vault, &alice, "Maison Alice", "FR", "100.01");
    let (bob_asset, _) = owned_asset(&mut vault, &bob, "Maison Benoît", "CH", "200");
    if dated {
        for asset in [&alice_asset, &bob_asset] {
            vault
                .save_profile_period(SaveProfilePeriodInput {
                    id: None,
                    expected_revision_id: None,
                    reason: "Pays historique explicitement daté".into(),
                    target_kind: "asset".into(),
                    target_id: asset.id.clone(),
                    profile_revision_id: asset.revision_id.clone(),
                    valid_from: "2024-01-01".into(),
                    valid_to: None,
                })
                .unwrap();
        }
    }
    let debt = vault
        .save_debt_contract(SaveDebtContractInput {
            id: None,
            expected_revision_id: None,
            reason: "Prêt interne synthétique".into(),
            name: "Prêt Alice vers Benoît".into(),
            currency: "EUR".into(),
            tracking_start: "2024-01-01".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    vault
        .save_debt_terms(SaveDebtTermsInput {
            id: None,
            expected_revision_id: None,
            reason: "Deux faces du même contrat".into(),
            debt_id: debt.id.clone(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
            principal_status: "complete".into(),
            interest_rate: None,
            interest_convention: String::new(),
            parties: vec![
                FinancingParty {
                    line_id: "creditor".into(),
                    side: "creditor".into(),
                    entity_id: alice.id.clone(),
                    legal_share: Some(fraction("1", "1")),
                    economic_share: Some(fraction("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
                FinancingParty {
                    line_id: "debtor".into(),
                    side: "debtor".into(),
                    entity_id: bob.id.clone(),
                    legal_share: Some(fraction("1", "1")),
                    economic_share: Some(fraction("1", "1")),
                    liability_kind: "single".into(),
                    notes: String::new(),
                },
            ],
            notes: String::new(),
        })
        .unwrap();
    vault
        .save_debt_movement(SaveDebtMovementInput {
            id: None,
            expected_revision_id: None,
            reason: "Stock historique confirmé, sans nouveau cash".into(),
            debt_id: debt.id.clone(),
            kind: "opening".into(),
            effective_on: "2024-01-01".into(),
            principal_change: "100.05".into(),
            interest_paid: "0".into(),
            fees_paid: "0".into(),
            unlinked_reason: "Ouverture historique du contrat synthétique".into(),
            notes: String::new(),
        })
        .unwrap();
    coverage(&mut vault);
    Fixture {
        temporary,
        vault,
        alice,
        bob,
        alice_asset,
        alice_value,
        debt,
    }
}

fn active_db(root: &Path) -> PathBuf {
    root.join("generations")
        .join(fs::read_to_string(root.join("CURRENT")).unwrap().trim())
        .join("patrimoine.sqlite3")
}

fn records(root: &Path) -> BTreeMap<String, Vec<Value>> {
    let c = Connection::open(active_db(root)).unwrap();
    let tables = c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap().query_map([], |r| r.get::<_, String>(0)).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap();
    tables
        .into_iter()
        .map(|name| {
            let mut q = c.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
            let count = q.column_count();
            let mut rows = q
                .query_map([], |row| {
                    Ok(Value::Array(
                        (0..count)
                            .map(|index| match row.get_ref(index).unwrap() {
                                ValueRef::Null => Value::Null,
                                ValueRef::Integer(n) => json!(n),
                                ValueRef::Text(s) => json!(std::str::from_utf8(s).unwrap()),
                                _ => panic!("Unexpected persistent storage class"),
                            })
                            .collect(),
                    ))
                })
                .unwrap()
                .collect::<rusqlite::Result<Vec<_>>>()
                .unwrap();
            rows.sort_by_cached_key(Value::to_string);
            (name, rows)
        })
        .collect()
}

#[test]
fn empty_snapshot_at_k_zero_preserves_unknown_coverage_instead_of_complete_zero() {
    let temporary = TempDir::new().unwrap();
    let vault = Vault::open(temporary.path().join("vault")).unwrap();
    let before = records(temporary.path().join("vault").as_path());
    let mut query = request();
    query.knowledge_sequence = Some(0);
    let report = vault.get_dashboard(query).unwrap();
    assert_eq!(report.knowledge_sequence, 0);
    assert_eq!(report.request.knowledge_sequence, Some(0));
    assert!(report.positions.is_empty());
    let totals = report.totals.unwrap();
    assert_eq!(totals.known_assets_eur, "0.00");
    assert_eq!(totals.known_liabilities_eur, "0.00");
    assert_eq!(totals.net_eur, None);
    assert!(report.warnings.iter().any(|w| w.code.contains("coverage")));
    assert!(report.history.iter().all(|p| p.totals.net_eur.is_none()));
    assert_eq!(records(temporary.path().join("vault").as_path()), before);
}

#[test]
fn residual_positions_and_members_reconcile_without_counting_internal_loan_twice() {
    let f = fixture();
    let before = records(f.temporary.path().join("vault").as_path());
    let report = f.vault.get_dashboard(request()).unwrap();
    let totals = report.totals.as_ref().unwrap();
    assert_eq!(totals.known_assets_eur, "300.01");
    assert_eq!(totals.known_liabilities_eur, "0.00");
    assert_eq!(totals.known_net_eur, "300.01");
    assert_eq!(totals.exact_known_assets, fraction("30001", "100"));
    assert_eq!(totals.exact_known_liabilities, fraction("0", "1"));
    let original = f
        .vault
        .get_financial_report(FinancialReportRequest {
            context: "family".into(),
            person_id: None,
            date: "2025-03-31".into(),
            knowledge_sequence: Some(report.knowledge_sequence),
        })
        .unwrap();
    assert_eq!(
        serde_json::to_value(report.financial.as_ref().unwrap()).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    let loan: Vec<_> = report
        .positions
        .iter()
        .filter(|p| p.position.object_id == f.debt.id)
        .collect();
    assert_eq!(loan.len(), 2);
    assert!(loan.iter().all(|p| p.exact_eur == Some(fraction("0", "1"))));
    assert!(loan
        .iter()
        .all(|p| p.exact_eliminated == fraction("2001", "20")));
    assert_eq!(
        report
            .members
            .iter()
            .find(|m| m.entity_id == f.alice.id)
            .unwrap()
            .totals
            .known_net_eur,
        "200.06"
    );
    assert_eq!(
        report
            .members
            .iter()
            .find(|m| m.entity_id == f.bob.id)
            .unwrap()
            .totals
            .known_net_eur,
        "99.95"
    );
    assert_eq!(report.history.first().unwrap().date, "2025-01-01");
    assert_eq!(report.history.last().unwrap().date, "2025-03-31");
    assert!(report
        .history
        .iter()
        .all(|p| p.totals.exact_known_net == fraction("30001", "100")));
    assert_eq!(
        report.period_change.unwrap().exact_known_net_change,
        fraction("0", "1")
    );
    assert_eq!(records(f.temporary.path().join("vault").as_path()), before);
}

#[test]
fn filters_select_residual_positions_and_reconcile_each_group_with_the_filtered_total() {
    let f = fixture();
    let mut query = request();
    query.filters.country = Some("FR".into());
    query.filters.currency = Some("EUR".into());
    query.filters.owner_entity_id = Some(f.alice.id.clone());
    query.filters.query = "maison alice".into();
    let report = f.vault.get_dashboard(query).unwrap();
    assert!(report.filters_applied);
    assert_eq!(report.positions.len(), 1);
    assert_eq!(report.positions[0].position.object_id, f.alice_asset.id);
    assert_eq!(report.positions[0].amount_eur.as_deref(), Some("100.01"));
    let totals = report.totals.unwrap();
    assert_eq!(totals.known_assets_eur, "100.01");
    for buckets in [&report.by_category, &report.by_country, &report.by_currency] {
        assert_eq!(buckets.len(), 1);
        assert_eq!(buckets[0].exact_assets, totals.exact_known_assets);
        assert_eq!(buckets[0].exact_liabilities, totals.exact_known_liabilities);
    }
    assert_eq!(report.by_country[0].key, "FR");
    assert_eq!(report.by_currency[0].key, "EUR");
    assert!(report
        .history
        .iter()
        .all(|p| p.totals.known_assets_eur == "100.01"));
}

#[test]
fn single_date_dashboard_keeps_one_stock_point_without_inventing_a_rental_period() {
    let f = fixture();
    let mut query = request();
    query.from_date = "2025-01-01".into();
    query.date = "2025-01-01".into();
    let report = f.vault.get_dashboard(query).unwrap();
    assert_eq!(report.history.len(), 1);
    assert_eq!(report.history[0].date, "2025-01-01");
    assert_eq!(report.history[0].totals.known_net_eur, "300.01");
    assert_eq!(report.totals.unwrap().known_net_eur, "300.01");
    assert!(report.rental.is_none());
}

#[test]
fn corrected_value_changes_current_dashboard_without_rewriting_pinned_history() {
    let mut f = fixture();
    let old = f.vault.get_dashboard(request()).unwrap();
    let mut historical = request();
    historical.knowledge_sequence = Some(old.knowledge_sequence);
    f.vault
        .save_valuation(SaveValuationInput {
            id: Some(f.alice_value.id.clone()),
            expected_revision_id: Some(f.alice_value.revision_id.clone()),
            reason: "Correction motivée conservant les anciennes connaissances".into(),
            target_kind: "asset".into(),
            target_id: f.alice_asset.id,
            amount: "150.11".into(),
            currency: "EUR".into(),
            effective_on: "2025-01-01".into(),
            source: "Estimation corrigée synthétique".into(),
        })
        .unwrap();
    let current = f.vault.get_dashboard(request()).unwrap();
    assert_eq!(current.totals.unwrap().known_assets_eur, "350.11");
    assert!(current
        .history
        .iter()
        .all(|p| p.totals.known_assets_eur == "350.11"));
    assert!(current.knowledge_sequence > old.knowledge_sequence);
    let restored = f.vault.get_dashboard(historical).unwrap();
    assert_eq!(
        serde_json::to_value(restored).unwrap(),
        serde_json::to_value(old).unwrap()
    );
}

#[test]
fn person_dashboard_preserves_large_source_precision_beyond_binary_float_capacity() {
    let mut f = fixture();
    f.vault
        .save_valuation(SaveValuationInput {
            id: Some(f.alice_value.id),
            expected_revision_id: Some(f.alice_value.revision_id),
            reason: "Précision exacte de la source".into(),
            target_kind: "asset".into(),
            target_id: f.alice_asset.id.clone(),
            amount: "12345678901234567890.1234".into(),
            currency: "EUR".into(),
            effective_on: "2025-01-01".into(),
            source: "Source décimale textuelle".into(),
        })
        .unwrap();
    let mut query = request();
    query.context = "person".into();
    query.entity_id = Some(f.alice.id.clone());
    query.filters.query = "Maison Alice".into();
    let report = f.vault.get_dashboard(query).unwrap();
    assert_eq!(report.positions.len(), 1);
    assert_eq!(
        report.positions[0].exact_eur,
        Some(fraction("61728394506172839450617", "5000"))
    );
    assert_eq!(
        report.totals.unwrap().known_assets_eur,
        "12345678901234567890.12"
    );
    assert!(report.members.is_empty());
}

#[test]
fn company_dashboard_reuses_company_report_without_turning_observations_into_net_assets() {
    let mut f = fixture();
    let company = entity(&mut f.vault, "Société de pilotage synthétique", "company");
    f.vault
        .save_company_policy(SaveCompanyPolicyInput {
            id: None,
            expected_revision_id: None,
            reason: "Méthode société explicite".into(),
            company_id: company.id.clone(),
            mode: "detailed".into(),
            coverage_complete: true,
            liability_qualified: true,
            valid_from: "2024-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    owned_asset(
        &mut f.vault,
        &company,
        "Actif propre de société",
        "FR",
        "350",
    );
    f.vault
        .save_company_financial_period(SaveCompanyFinancialPeriodInput {
            id: None,
            expected_revision_id: None,
            reason: "Observation hors valorisation patrimoniale".into(),
            company_id: company.id.clone(),
            period_start: "2024-01-01".into(),
            period_end: "2025-01-01".into(),
            frequency: "annual".into(),
            currency: "EUR".into(),
            revenue: Some("100000".into()),
            operating_expenses: None,
            operating_result: None,
            net_result: None,
            indicative_cash: Some("999999".into()),
            indicative_debt: None,
            important_receivables: None,
            dividends_declared: None,
            indicative_global_value: None,
            notes: String::new(),
        })
        .unwrap();
    let mut query = request();
    query.context = "company".into();
    query.entity_id = Some(company.id.clone());
    query.from_date = "2024-01-01".into();
    let report = f.vault.get_dashboard(query).unwrap();
    assert!(report.financial.is_none());
    assert!(report.totals.is_none());
    assert!(report.history.is_empty());
    assert!(report.members.is_empty());
    let detail = report.company.unwrap();
    assert_eq!(detail.company_id, company.id);
    assert_eq!(
        detail.financial_periods[0]
            .period
            .indicative_cash
            .as_deref(),
        Some("999999")
    );
    let original = f
        .vault
        .get_company_report(CompanyReportRequest {
            company_id: company.id,
            date: "2025-03-31".into(),
            frequency: None,
            knowledge_sequence: Some(report.knowledge_sequence),
        })
        .unwrap();
    assert_eq!(
        serde_json::to_value(detail).unwrap(),
        serde_json::to_value(original).unwrap()
    );
}

#[test]
fn company_period_window_keeps_complete_parallel_observations_and_distributions_at_fixed_k() {
    let mut f = fixture();
    let company = entity(&mut f.vault, "Société aux observations datées", "company");
    let period_input =
        |start: &str, end: &str, frequency: &str, revenue: &str| SaveCompanyFinancialPeriodInput {
            id: None,
            expected_revision_id: None,
            reason: "Observation source sans prorata".into(),
            company_id: company.id.clone(),
            period_start: start.into(),
            period_end: end.into(),
            frequency: frequency.into(),
            currency: "EUR".into(),
            revenue: Some(revenue.into()),
            operating_expenses: None,
            operating_result: None,
            net_result: None,
            indicative_cash: None,
            indicative_debt: None,
            important_receivables: None,
            dividends_declared: None,
            indicative_global_value: None,
            notes: String::new(),
        };
    let annual = f
        .vault
        .save_company_financial_period(period_input("2024-01-01", "2025-01-01", "annual", "120"))
        .unwrap();
    let monthly = f
        .vault
        .save_company_financial_period(period_input("2025-01-01", "2025-02-01", "monthly", "10.25"))
        .unwrap();
    f.vault
        .save_company_financial_period(period_input("2024-07-01", "2024-10-01", "quarterly", "30"))
        .unwrap();
    f.vault
        .save_company_financial_period(period_input("2025-02-01", "2025-03-01", "monthly", "11"))
        .unwrap();
    for date in ["2024-12-30", "2024-12-31", "2025-02-15", "2025-02-16"] {
        f.vault
            .save_company_distribution(SaveCompanyDistributionInput {
                id: None,
                expected_revision_id: None,
                reason: "Décision de distribution datée".into(),
                company_id: company.id.clone(),
                beneficiary_entity_id: f.alice.id.clone(),
                kind: "declared".into(),
                effective_on: date.into(),
                amount: "5".into(),
                currency: "EUR".into(),
                related_declaration_id: None,
                unlinked_reason: String::new(),
                notes: String::new(),
                cash_links: Vec::new(),
            })
            .unwrap();
    }
    let old_k = f.vault.list_registry().unwrap().knowledge_sequence;
    let mut correction = period_input("2024-01-01", "2025-01-01", "annual", "135");
    correction.id = Some(annual.id.clone());
    correction.expected_revision_id = Some(annual.revision_id.clone());
    let corrected = f.vault.save_company_financial_period(correction).unwrap();
    let before_reads = records(f.temporary.path().join("vault").as_path());
    let mut query = request();
    query.context = "company".into();
    query.entity_id = Some(company.id);
    query.from_date = "2024-12-31".into();
    query.date = "2025-02-15".into();
    query.knowledge_sequence = Some(old_k);
    let old = f.vault.get_dashboard(query.clone()).unwrap();
    let details = old.company.as_ref().unwrap();
    assert_eq!(details.financial_periods.len(), 2);
    assert_eq!(
        details
            .financial_periods
            .iter()
            .find(|p| p.period.id == annual.id)
            .unwrap()
            .period
            .revenue
            .as_deref(),
        Some("120")
    );
    assert_eq!(
        details
            .financial_periods
            .iter()
            .find(|p| p.period.id == monthly.id)
            .unwrap()
            .period
            .revenue
            .as_deref(),
        Some("10.25")
    );
    assert_eq!(
        details
            .financial_periods
            .iter()
            .map(|p| p.period.frequency.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["annual", "monthly"])
    );
    assert!(!old.source_ids.contains(&corrected.revision_id));
    assert_eq!(
        details
            .distributions
            .iter()
            .map(|p| p.distribution.effective_on.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from(["2024-12-31", "2025-02-15"])
    );
    let export = f.temporary.path().join("societe-periode.json");
    f.vault
        .export_dashboard_report(query.clone(), &export)
        .unwrap();
    let exported: Value = serde_json::from_slice(&fs::read(export).unwrap()).unwrap();
    assert_eq!(
        exported["report"]["company"],
        serde_json::to_value(details).unwrap()
    );
    assert_eq!(exported["report"]["request"]["knowledge_sequence"], old_k);
    query.knowledge_sequence = None;
    let current = f.vault.get_dashboard(query.clone()).unwrap();
    assert_eq!(
        current
            .company
            .unwrap()
            .financial_periods
            .iter()
            .find(|p| p.period.id == annual.id)
            .unwrap()
            .period
            .revenue
            .as_deref(),
        Some("135")
    );
    query.from_date = query.date.clone();
    let one_date = f.vault.get_dashboard(query).unwrap().company.unwrap();
    assert!(one_date.financial_periods.is_empty());
    assert_eq!(one_date.distributions.len(), 1);
    assert_eq!(
        one_date.distributions[0].distribution.effective_on,
        "2025-02-15"
    );
    assert_eq!(
        records(f.temporary.path().join("vault").as_path()),
        before_reads
    );
}

#[test]
fn filtered_report_export_is_private_complete_json_and_does_not_mutate_the_vault() {
    let mut f = fixture();
    let before = records(f.temporary.path().join("vault").as_path());
    let mut query = request();
    query.filters.country = Some("FR".into());
    query.filters.query = "Maison Alice".into();
    let destination = f.temporary.path().join("rapport-filtre.json");
    let expected = f.vault.get_dashboard(query.clone()).unwrap();
    assert!(expected
        .financial
        .as_ref()
        .unwrap()
        .positions
        .iter()
        .any(|position| position.label == "Maison Benoît"));
    f.vault
        .export_dashboard_report(query, &destination)
        .unwrap();
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let bytes = fs::read(&destination).unwrap();
    let exported: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(exported["report_kind"], "patrimoine_filtered_report");
    assert_eq!(exported["format_version"], 1);
    assert_eq!(exported["schema_version"], 6);
    assert_eq!(
        exported["excluded_sections"],
        json!(["financial", "rental", "variation"])
    );
    assert!(exported["selection_note"]
        .as_str()
        .unwrap()
        .contains("non filtrés"));
    assert!(exported["report"]["financial"].is_null());
    assert!(exported["report"]["rental"].is_null());
    assert!(exported["report"]["variation"].is_null());
    assert_eq!(exported["report"]["positions"].as_array().unwrap().len(), 1);
    assert_eq!(
        exported["report"]["positions"][0]["position"]["label"],
        "Maison Alice"
    );
    assert_eq!(exported["report"]["totals"]["known_assets_eur"], "100.01");
    assert!(!std::str::from_utf8(&bytes)
        .unwrap()
        .contains("Maison Benoît"));
    assert_eq!(
        exported["report"]["request"]["knowledge_sequence"],
        expected.knowledge_sequence
    );
    assert_eq!(
        exported["report"]["positions"],
        serde_json::to_value(&expected.positions).unwrap()
    );
    assert_eq!(
        exported["report"]["history"],
        serde_json::to_value(&expected.history).unwrap()
    );
    assert_eq!(
        exported["report"]["members"],
        serde_json::to_value(&expected.members).unwrap()
    );
    // Export scoping does not change the interactive source explanations.
    assert_eq!(
        serde_json::to_value(f.vault.get_dashboard(expected.request.clone()).unwrap()).unwrap(),
        serde_json::to_value(expected).unwrap()
    );
    assert_eq!(records(f.temporary.path().join("vault").as_path()), before);
}

#[test]
fn an_unfiltered_report_export_keeps_the_complete_scope_explanations() {
    let mut f = fixture();
    let before = records(f.temporary.path().join("vault").as_path());
    let expected = f.vault.get_dashboard(request()).unwrap();
    let target = f.temporary.path().join("rapport-sans-filtre.json");
    f.vault
        .export_dashboard_report(expected.request.clone(), &target)
        .unwrap();
    let exported: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
    assert_eq!(exported["excluded_sections"], json!([]));
    assert!(exported["selection_note"].is_null());
    assert!(exported["report"]["financial"].is_object());
    assert!(exported["report"]["rental"].is_object());
    assert!(exported["report"]["variation"].is_object());
    assert_eq!(exported["report"], serde_json::to_value(expected).unwrap());
    assert_eq!(records(f.temporary.path().join("vault").as_path()), before);
}

#[test]
fn invalid_requests_and_destinations_leave_no_partial_report_or_vault_change() {
    let mut f = fixture();
    let before = records(f.temporary.path().join("vault").as_path());
    let mut invalid_requests = Vec::new();
    let mut bad = request();
    bad.knowledge_sequence = Some(u64::MAX);
    invalid_requests.push(bad);
    let mut bad = request();
    bad.context = "person".into();
    bad.entity_id = Some("absent".into());
    invalid_requests.push(bad);
    let mut bad = request();
    bad.filters.owner_entity_id = Some("absent".into());
    invalid_requests.push(bad);
    let mut bad = request();
    bad.date = "2025-02-29".into();
    invalid_requests.push(bad);
    let mut bad = request();
    bad.from_date = "2030-01-01".into();
    invalid_requests.push(bad);
    let mut bad = request();
    bad.interval = "daily".into();
    invalid_requests.push(bad);
    let mut bad = request();
    bad.from_date = "2000-01-01".into();
    invalid_requests.push(bad);
    for (index, query) in invalid_requests.into_iter().enumerate() {
        let output = f.temporary.path().join(format!("invalid-{index}.json"));
        assert!(f.vault.export_dashboard_report(query, &output).is_err());
        assert!(!output.exists());
    }
    let existing = f.temporary.path().join("existing.json");
    fs::write(&existing, b"conserver exactement").unwrap();
    assert!(f
        .vault
        .export_dashboard_report(request(), &existing)
        .is_err());
    assert_eq!(fs::read(&existing).unwrap(), b"conserver exactement");
    let alias = f.temporary.path().join("alias.json");
    symlink(&existing, &alias).unwrap();
    assert!(f.vault.export_dashboard_report(request(), &alias).is_err());
    let inside = f.temporary.path().join("vault/inside.json");
    assert!(f.vault.export_dashboard_report(request(), &inside).is_err());
    assert!(!inside.exists());
    let missing_parent = f.temporary.path().join("missing/report.json");
    assert!(f
        .vault
        .export_dashboard_report(request(), &missing_parent)
        .is_err());
    assert!(!missing_parent.parent().unwrap().exists());
    let wrong_extension = f.temporary.path().join("report.zip");
    assert!(f
        .vault
        .export_dashboard_report(request(), &wrong_extension)
        .is_err());
    assert!(!wrong_extension.exists());
    assert_eq!(records(f.temporary.path().join("vault").as_path()), before);
}

#[test]
fn an_undated_country_remains_unknown_until_a_profile_is_explicit_at_the_selected_k() {
    let mut f = fixture_with_profiles(false);
    let before = f.vault.list_registry().unwrap().knowledge_sequence;
    let report = f.vault.get_dashboard(request()).unwrap();
    let position = report
        .positions
        .iter()
        .find(|p| p.position.object_id == f.alice_asset.id)
        .unwrap();
    assert_eq!(position.country, None);
    let mut query = request();
    query.filters.country = Some("FR".into());
    assert!(f
        .vault
        .get_dashboard(query.clone())
        .unwrap()
        .positions
        .is_empty());
    f.vault
        .save_profile_period(SaveProfilePeriodInput {
            id: None,
            expected_revision_id: None,
            reason: "Correction explicite de la datation du pays".into(),
            target_kind: "asset".into(),
            target_id: f.alice_asset.id.clone(),
            profile_revision_id: f.alice_asset.revision_id.clone(),
            valid_from: "2024-01-01".into(),
            valid_to: None,
        })
        .unwrap();
    let current = f.vault.get_dashboard(query.clone()).unwrap();
    assert_eq!(current.positions.len(), 1);
    assert_eq!(current.positions[0].amount_eur.as_deref(), Some("100.01"));
    query.knowledge_sequence = Some(before);
    assert!(f.vault.get_dashboard(query).unwrap().positions.is_empty());
}

#[test]
fn a_company_without_a_financial_history_does_not_consume_the_monthly_point_budget() {
    let mut f = fixture();
    let company = entity(&mut f.vault, "Société sur une longue fenêtre", "company");
    let mut query = request();
    query.context = "company".into();
    query.entity_id = Some(company.id);
    query.from_date = "2019-01-01".into();
    query.interval = "monthly".into();
    let report = f.vault.get_dashboard(query).unwrap();
    assert!(report.history.is_empty());
    assert!(report.company.is_some());
}
