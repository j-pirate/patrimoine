//! Read-only projections of already calculated financial reports.
//!
//! Elimination is allocated to the original faces before filtering. This is a
//! presentation projection, never a new consolidation or a performance engine.
use crate::exact::{format_eur, fraction_of};
use crate::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use time::{Date, Month};

pub const DASHBOARD_ENGINE_VERSION: &str = "dashboard-exact-1";
pub const DASHBOARD_MAX_POINTS: usize = 61;
const CATEGORIES: [&str; 12] = [
    "immobilier",
    "banque",
    "especes",
    "or",
    "argent",
    "participations",
    "creances",
    "financements",
    "depots",
    "avances",
    "decouverts",
    "autres",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DashboardFilters {
    pub country: Option<String>,
    pub currency: Option<String>,
    pub category: Option<String>,
    pub owner_entity_id: Option<String>,
    #[serde(default)]
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DashboardRequest {
    pub context: String,
    pub entity_id: Option<String>,
    pub date: String,
    pub from_date: String,
    pub interval: String,
    pub knowledge_sequence: Option<u64>,
    #[serde(default)]
    pub filters: DashboardFilters,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardTotals {
    pub known_assets_eur: String,
    pub known_liabilities_eur: String,
    pub known_net_eur: String,
    pub exact_known_assets: Fraction,
    pub exact_known_liabilities: Fraction,
    pub exact_known_net: Fraction,
    pub assets_eur: Option<String>,
    pub liabilities_eur: Option<String>,
    pub net_eur: Option<String>,
    pub position_count: usize,
    pub unknown_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPosition {
    pub position: FinancialPosition,
    pub side: String,
    pub category: String,
    pub country: Option<String>,
    pub owner_entity_ids: Vec<String>,
    pub amount_eur: Option<String>,
    pub exact_eur: Option<Fraction>,
    pub eliminated_eur: String,
    pub exact_eliminated: Fraction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardBucket {
    pub key: String,
    pub label: String,
    pub known_assets_eur: String,
    pub known_liabilities_eur: String,
    pub known_net_eur: String,
    pub exact_assets: Fraction,
    pub exact_liabilities: Fraction,
    pub exact_net: Fraction,
    pub position_count: usize,
    pub unknown_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardMember {
    pub entity_id: String,
    pub label: String,
    pub totals: DashboardTotals,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPoint {
    pub date: String,
    pub status: String,
    pub totals: DashboardTotals,
    pub source_ids: Vec<String>,
    pub warnings: Vec<FinancialWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardChange {
    pub from_date: String,
    pub to_date: String,
    pub known_net_change_eur: String,
    pub exact_known_net_change: Fraction,
    pub net_change_eur: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardReport {
    pub request: DashboardRequest,
    pub knowledge_sequence: u64,
    pub engine_version: String,
    pub financial: Option<FinancialReport>,
    pub company: Option<CompanyReport>,
    pub rental: Option<RentalReport>,
    pub totals: Option<DashboardTotals>,
    pub positions: Vec<DashboardPosition>,
    pub by_category: Vec<DashboardBucket>,
    pub by_country: Vec<DashboardBucket>,
    pub by_currency: Vec<DashboardBucket>,
    pub members: Vec<DashboardMember>,
    pub history: Vec<DashboardPoint>,
    pub period_change: Option<DashboardChange>,
    pub variation: Option<VariationReport>,
    pub warnings: Vec<FinancialWarning>,
    pub source_ids: Vec<String>,
    pub filters_applied: bool,
}

/// All snapshots were read at one K, in one database read transaction.
#[derive(Clone, Copy)]
pub struct DashboardSources<'a> {
    pub registry: &'a RegistrySnapshot,
    pub properties: &'a PropertySnapshot,
    pub treasury: &'a TreasurySnapshot,
    pub metals: &'a MetalSnapshot,
}

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn integrity(message: &str) -> AppError {
    AppError::new("integrity", message)
}
fn ratio(value: &Fraction) -> Result<BigRational> {
    if value.num.len() > 40_000 || value.den.len() > 40_000 {
        return Err(integrity(
            "Le résultat du tableau de bord dépasse la précision autorisée.",
        ));
    }
    let numerator = value
        .num
        .parse::<BigInt>()
        .map_err(|_| integrity("Un résultat exact du tableau de bord est invalide."))?;
    let denominator = value
        .den
        .parse::<BigInt>()
        .map_err(|_| integrity("Un résultat exact du tableau de bord est invalide."))?;
    if denominator <= BigInt::zero() {
        return Err(integrity(
            "Un dénominateur du tableau de bord est invalide.",
        ));
    }
    Ok(BigRational::new(numerator, denominator))
}
fn filtered(filters: &DashboardFilters) -> bool {
    filters.country.is_some()
        || filters.currency.is_some()
        || filters.category.is_some()
        || filters.owner_entity_id.is_some()
        || !filters.query.trim().is_empty()
}
fn identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn validate_dashboard_request(request: &DashboardRequest) -> Result<()> {
    validate_date(&request.date)?;
    validate_date(&request.from_date)?;
    if request.from_date > request.date {
        return Err(invalid("Le début de la période doit précéder sa fin."));
    }
    if !["monthly", "annual"].contains(&request.interval.as_str()) {
        return Err(invalid("Choisissez une observation mensuelle ou annuelle."));
    }
    if !((request.context == "family" && request.entity_id.is_none())
        || (["person", "company"].contains(&request.context.as_str())
            && request.entity_id.as_deref().is_some_and(identifier)))
    {
        return Err(invalid("Le périmètre du tableau de bord est invalide."));
    }
    let f = &request.filters;
    if f.country
        .as_ref()
        .is_some_and(|v| v.len() != 2 || !v.bytes().all(|b| b.is_ascii_uppercase()))
        || f.currency
            .as_ref()
            .is_some_and(|v| v.len() != 3 || !v.bytes().all(|b| b.is_ascii_uppercase()))
        || f.category
            .as_ref()
            .is_some_and(|v| !CATEGORIES.contains(&v.as_str()))
        || f.owner_entity_id.as_deref().is_some_and(|v| !identifier(v))
        || f.query.chars().count() > 200
        || f.query.chars().any(char::is_control)
    {
        return Err(invalid("Un filtre du tableau de bord est invalide."));
    }
    if request.context == "company" && filtered(f) {
        return Err(invalid("Les filtres de positions concernent les bilans famille et membre. Effacez-les pour consulter une société."));
    }
    Ok(())
}

fn date(value: &str) -> Date {
    // validate_dashboard_request checked the complete civil date first.
    Date::from_calendar_date(
        value[..4].parse().unwrap(),
        Month::try_from(value[5..7].parse::<u8>().unwrap()).unwrap(),
        value[8..].parse().unwrap(),
    )
    .unwrap()
}
fn civil(value: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        value.year(),
        u8::from(value.month()),
        value.day()
    )
}
fn period_end(value: Date, interval: &str) -> Option<Date> {
    if interval == "annual" {
        return Date::from_calendar_date(value.year(), Month::December, 31).ok();
    }
    let (year, month) = if value.month() == Month::December {
        (value.year() + 1, Month::January)
    } else {
        (value.year(), value.month().next())
    };
    Date::from_calendar_date(year, month, 1)
        .ok()?
        .previous_day()
}

/// Exact endpoints plus calendar month/year ends; every observation uses the
/// same K. A long interval is rejected explicitly rather than silently sampled.
pub fn dashboard_dates(request: &DashboardRequest) -> Result<Vec<String>> {
    validate_dashboard_request(request)?;
    let first = date(&request.from_date);
    let last = date(&request.date);
    let mut dates = vec![request.from_date.clone()];
    let mut cursor = first;
    while cursor < last {
        let end = period_end(cursor, &request.interval).unwrap_or(last);
        if end > cursor && end < last {
            dates.push(civil(end));
        }
        if end >= last {
            break;
        }
        let Some(next) = end.next_day() else { break };
        cursor = next;
        if dates.len() >= DASHBOARD_MAX_POINTS {
            return Err(invalid("La période dépasse 61 observations. Réduisez-la ou choisissez une fréquence annuelle."));
        }
    }
    if dates.last() != Some(&request.date) {
        dates.push(request.date.clone());
    }
    if dates.len() > DASHBOARD_MAX_POINTS {
        return Err(invalid(
            "La période dépasse 61 observations. Réduisez-la ou choisissez une fréquence annuelle.",
        ));
    }
    Ok(dates)
}

fn side(kind: &str) -> Result<&'static str> {
    Ok(match kind {
        "guarantee" => "contingent",
        "overdraft" | "loan_debt" | "rent_debt" | "deposit_liability" | "advance_liability" => {
            "liability"
        }
        "asset" | "global_equity" | "metal" | "treasury" | "loan_receivable"
        | "rent_receivable" | "deposit_receivable" | "advance_receivable" => "asset",
        _ => return Err(integrity(
            "La nature d’une position financière n’est pas prise en charge par le tableau de bord.",
        )),
    })
}
fn category(position: &FinancialPosition, sources: DashboardSources<'_>, on: &str) -> String {
    match position.kind.as_str() {
        "global_equity" => "participations",
        "metal" => sources
            .metals
            .lots
            .iter()
            .find(|l| l.id == position.object_id)
            .map(|l| if l.metal == "gold" { "or" } else { "argent" })
            .unwrap_or("autres"),
        "treasury" => sources
            .treasury
            .accounts
            .iter()
            .find(|a| a.id == position.object_id)
            .map(|a| {
                if a.kind == "cash" {
                    "especes"
                } else {
                    "banque"
                }
            })
            .unwrap_or("autres"),
        "overdraft" => "decouverts",
        "loan_receivable" | "rent_receivable" => "creances",
        "loan_debt" | "rent_debt" => "financements",
        "deposit_receivable" | "deposit_liability" => "depots",
        "advance_receivable" | "advance_liability" => "avances",
        "asset"
            if sources.properties.properties.iter().any(|p| {
                p.asset_id == position.object_id
                    && applies(&p.valid_from, p.valid_to.as_deref(), on)
            }) || sources.registry.assets.iter().any(|a| {
                a.id == position.object_id
                    && ["immobilier", "real_estate"].contains(&a.category.as_str())
            }) =>
        {
            "immobilier"
        }
        _ => "autres",
    }
    .into()
}

fn annotate(
    position: &FinancialPosition,
    sources: DashboardSources<'_>,
    on: &str,
) -> Result<DashboardPosition> {
    let registry = sources.registry;
    let mut owners = BTreeSet::new();
    if let Some(owner) = &position.entity_id {
        owners.insert(owner.clone());
    }
    let mut asset_id = position.object_id.as_str();
    if let Some(account) = sources
        .treasury
        .accounts
        .iter()
        .find(|a| a.id == position.object_id)
    {
        asset_id = &account.asset_id;
    } else if let Some(lot) = sources
        .metals
        .lots
        .iter()
        .find(|l| l.id == position.object_id)
    {
        asset_id = &lot.asset_id;
    }
    owners.extend(
        registry
            .ownerships
            .iter()
            .filter(|o| o.asset_id == asset_id && applies(&o.valid_from, o.valid_to.as_deref(), on))
            .map(|o| o.owner_entity_id.clone()),
    );
    if position.kind == "global_equity" {
        owners.extend(
            registry
                .participations
                .iter()
                .filter(|p| {
                    p.company_id == position.object_id
                        && applies(&p.valid_from, p.valid_to.as_deref(), on)
                })
                .map(|p| p.owner_entity_id.clone()),
        );
    }
    // The source engine resolves dated asset/entity profiles at D and K.
    // None is meaningful historical information, never permission to infer
    // a country from today's descriptive profile or another position face.
    let country = position.country.clone();
    let exact = position.exact_eur.as_ref().map(ratio).transpose()?;
    Ok(DashboardPosition {
        position: position.clone(),
        side: side(&position.kind)?.into(),
        category: category(position, sources, on),
        country,
        owner_entity_ids: owners.into_iter().collect(),
        amount_eur: exact.as_ref().map(format_eur),
        exact_eur: exact.as_ref().map(fraction_of),
        eliminated_eur: "0.00".into(),
        exact_eliminated: fraction_of(&BigRational::zero()),
    })
}

fn eliminate(
    positions: &mut [DashboardPosition],
    elimination: &FinancialElimination,
) -> Result<()> {
    let amount = ratio(&elimination.exact_eliminated)?;
    if amount < BigRational::zero() {
        return Err(integrity("Une élimination interne est négative."));
    }
    if amount.is_zero() {
        return Ok(());
    }
    for face in ["asset", "liability"] {
        let indices = positions
            .iter()
            .enumerate()
            .filter_map(|(i, p)| {
                (p.position.object_id == elimination.object_id && p.side == face)
                    .then_some((i, p.exact_eur.as_ref()))
            })
            .map(|(i, v)| Ok((i, v.map(ratio).transpose()?)))
            .collect::<Result<Vec<_>>>()?;
        let total = indices
            .iter()
            .filter_map(|(_, v)| v.as_ref())
            .filter(|v| v.is_positive())
            .fold(BigRational::zero(), |sum, v| sum + v);
        if total < amount {
            return Err(integrity(
                "Une élimination interne dépasse les positions de sa face.",
            ));
        }
        for (index, value) in indices {
            let Some(value) = value.filter(|v| v.is_positive()) else {
                continue;
            };
            let reduction = &amount * &value / &total;
            let position = &mut positions[index];
            let after = value - &reduction;
            let eliminated = ratio(&position.exact_eliminated)? + reduction;
            position.amount_eur = Some(format_eur(&after));
            position.exact_eur = Some(fraction_of(&after));
            position.eliminated_eur = format_eur(&eliminated);
            position.exact_eliminated = fraction_of(&eliminated);
            position
                .position
                .source_ids
                .extend(elimination.source_ids.iter().cloned());
            position.position.source_ids.sort();
            position.position.source_ids.dedup();
        }
    }
    Ok(())
}

fn matches(
    position: &DashboardPosition,
    filters: &DashboardFilters,
    registry: &RegistrySnapshot,
) -> bool {
    if filters
        .country
        .as_ref()
        .is_some_and(|v| position.country.as_ref() != Some(v))
        || filters
            .currency
            .as_ref()
            .is_some_and(|v| position.position.currency != *v)
        || filters
            .category
            .as_ref()
            .is_some_and(|v| position.category != *v)
        || filters
            .owner_entity_id
            .as_ref()
            .is_some_and(|v| !position.owner_entity_ids.contains(v))
    {
        return false;
    }
    let query = filters.query.trim().to_lowercase();
    if query.is_empty() {
        return true;
    }
    let mut texts = vec![
        position.position.label.as_str(),
        position.position.object_id.as_str(),
        position.position.kind.as_str(),
        position.category.as_str(),
        position.position.currency.as_str(),
    ];
    if let Some(country) = position.country.as_deref() {
        texts.push(country);
    }
    texts.extend(position.position.source_ids.iter().map(String::as_str));
    texts.extend(position.owner_entity_ids.iter().map(String::as_str));
    texts.extend(
        registry
            .entities
            .iter()
            .filter(|e| position.owner_entity_ids.contains(&e.id))
            .map(|e| e.display_name.as_str()),
    );
    texts.into_iter().any(|v| v.to_lowercase().contains(&query))
}

fn sums(positions: &[DashboardPosition]) -> Result<(BigRational, BigRational, usize, usize)> {
    let mut assets = BigRational::zero();
    let mut liabilities = BigRational::zero();
    let mut count = 0;
    let mut unknown = 0;
    for p in positions.iter().filter(|p| p.side != "contingent") {
        count += 1;
        match &p.exact_eur {
            Some(value) => {
                if p.side == "liability" {
                    liabilities += ratio(value)?
                } else {
                    assets += ratio(value)?
                }
            }
            None => unknown += 1,
        }
    }
    Ok((assets, liabilities, count, unknown))
}
fn totals(positions: &[DashboardPosition], report: &FinancialReport) -> Result<DashboardTotals> {
    let (a, l, count, unknown) = sums(positions)?;
    let net = &a - &l;
    Ok(DashboardTotals {
        known_assets_eur: format_eur(&a),
        known_liabilities_eur: format_eur(&l),
        known_net_eur: format_eur(&net),
        exact_known_assets: fraction_of(&a),
        exact_known_liabilities: fraction_of(&l),
        exact_known_net: fraction_of(&net),
        assets_eur: (report.assets_eur.is_some()
            && !positions
                .iter()
                .any(|p| p.side == "asset" && p.exact_eur.is_none()))
        .then(|| format_eur(&a)),
        liabilities_eur: (report.liabilities_eur.is_some()
            && !positions
                .iter()
                .any(|p| p.side == "liability" && p.exact_eur.is_none()))
        .then(|| format_eur(&l)),
        net_eur: (report.net_eur.is_some() && unknown == 0).then(|| format_eur(&net)),
        position_count: count,
        unknown_count: unknown,
    })
}
fn positions(
    sources: DashboardSources<'_>,
    report: &FinancialReport,
    filters: &DashboardFilters,
) -> Result<Vec<DashboardPosition>> {
    let registry = apply_profile_periods(sources.registry, &report.date)?;
    let sources = DashboardSources {
        registry: &registry,
        ..sources
    };
    let mut positions = report
        .positions
        .iter()
        .map(|p| annotate(p, sources, &report.date))
        .collect::<Result<Vec<_>>>()?;
    for e in &report.eliminations {
        eliminate(&mut positions, e)?;
    }
    let complete = totals(&positions, report)?;
    if complete.known_assets_eur != report.known_assets_eur
        || complete.known_liabilities_eur != report.known_liabilities_eur
        || complete.known_net_eur != report.known_net_eur
    {
        return Err(integrity(
            "Les positions résiduelles ne se réconcilient pas avec le bilan source.",
        ));
    }
    positions.retain(|p| matches(p, filters, &registry));
    // Source order is stable; ranking exact amounts needs no rounded text sort.
    positions.sort_by(|a, b| {
        let amount = |p: &DashboardPosition| {
            p.exact_eur
                .as_ref()
                .and_then(|f| ratio(f).ok())
                .map(|v| v.abs())
        };
        amount(b)
            .cmp(&amount(a))
            .then_with(|| a.position.object_id.cmp(&b.position.object_id))
            .then_with(|| a.side.cmp(&b.side))
    });
    Ok(positions)
}

fn category_label(key: &str) -> &str {
    match key {
        "immobilier" => "Immobilier",
        "banque" => "Comptes bancaires",
        "especes" => "Espèces",
        "or" => "Or",
        "argent" => "Argent métal",
        "participations" => "Participations globales",
        "creances" => "Créances",
        "financements" => "Financements et dettes locatives",
        "depots" => "Dépôts de garantie",
        "avances" => "Avances locatives",
        "decouverts" => "Découverts",
        _ => "Autres positions",
    }
}
fn buckets(positions: &[DashboardPosition], dimension: &str) -> Result<Vec<DashboardBucket>> {
    let mut grouped: BTreeMap<String, Vec<DashboardPosition>> = BTreeMap::new();
    for position in positions.iter().filter(|p| p.side != "contingent") {
        let key = match dimension {
            "country" => position.country.as_deref().unwrap_or("unknown"),
            "currency" => &position.position.currency,
            _ => &position.category,
        };
        grouped
            .entry(key.into())
            .or_default()
            .push(position.clone());
    }
    grouped
        .into_iter()
        .map(|(key, rows)| {
            let (a, l, count, unknown) = sums(&rows)?;
            let net = &a - &l;
            let label = if dimension == "category" {
                category_label(&key)
            } else if key == "unknown" {
                "Pays non renseigné"
            } else {
                &key
            };
            Ok(DashboardBucket {
                label: label.into(),
                key,
                known_assets_eur: format_eur(&a),
                known_liabilities_eur: format_eur(&l),
                known_net_eur: format_eur(&net),
                exact_assets: fraction_of(&a),
                exact_liabilities: fraction_of(&l),
                exact_net: fraction_of(&net),
                position_count: count,
                unknown_count: unknown,
            })
        })
        .collect()
}

fn check_financial(
    report: &FinancialReport,
    context: &str,
    entity: Option<&str>,
    on: &str,
    k: u64,
) -> Result<()> {
    if report.context != context
        || report.person_id.as_deref() != entity
        || report.date != on
        || report.knowledge_sequence != k
    {
        return Err(integrity(
            "Un bilan ne correspond pas au contexte du tableau de bord.",
        ));
    }
    Ok(())
}

fn financial_warnings(
    sources: DashboardSources<'_>,
    report: &FinancialReport,
) -> Vec<FinancialWarning> {
    let mut warnings = report.warnings.clone();
    let observed = date(&report.date);
    for position in &report.positions {
        for valuation in sources.registry.valuations.iter().filter(|v| {
            v.effective_on <= report.date && position.source_ids.contains(&v.revision_id)
        }) {
            let age = (observed - date(&valuation.effective_on)).whole_days();
            if age > 365 {
                warnings.push(FinancialWarning {
                    code: "stale_valuation".into(),
                    object_id: position.object_id.clone(),
                    message: format!(
                        "L’estimation du {} a {} jours, au-delà du seuil indicatif de 365 jours. La valeur sourcée reste inchangée. Source {} / révision {}.",
                        valuation.effective_on, age, valuation.id, valuation.revision_id
                    ),
                });
            }
        }
    }
    warnings.sort_by(|a, b| {
        (&a.code, &a.object_id, &a.message).cmp(&(&b.code, &b.object_id, &b.message))
    });
    warnings
        .dedup_by(|a, b| a.code == b.code && a.object_id == b.object_id && a.message == b.message);
    warnings
}

/// Assemble only reports computed from the provided snapshots. Separate inputs
/// keep the infrastructure responsible for the single read transaction, while
/// the exact projection and its validation remain independent of SQLite/Tauri.
#[allow(clippy::too_many_arguments)]
pub fn build_dashboard(
    sources: DashboardSources<'_>,
    mut request: DashboardRequest,
    financial: Option<FinancialReport>,
    company: Option<CompanyReport>,
    rental: Option<RentalReport>,
    members: Vec<(String, FinancialReport)>,
    history: Vec<FinancialReport>,
    variation: Option<VariationReport>,
) -> Result<DashboardReport> {
    validate_dashboard_request(&request)?;
    let k = sources.registry.knowledge_sequence;
    if [
        sources.properties.knowledge_sequence,
        sources.treasury.knowledge_sequence,
        sources.metals.knowledge_sequence,
    ]
    .iter()
    .any(|v| *v != k)
        || request.knowledge_sequence.is_some_and(|v| v != k)
    {
        return Err(integrity(
            "Les sources du tableau de bord ne partagent pas les mêmes connaissances.",
        ));
    }
    if request.context != "family"
        && !sources
            .registry
            .entities
            .iter()
            .any(|e| Some(&e.id) == request.entity_id.as_ref() && e.kind == request.context)
    {
        return Err(invalid(
            "L’entité du tableau de bord n’est pas connue à cette séquence.",
        ));
    }
    if request
        .filters
        .owner_entity_id
        .as_ref()
        .is_some_and(|id| !sources.registry.entities.iter().any(|e| e.id == *id))
    {
        return Err(invalid(
            "Le propriétaire filtré n’est pas connu à cette séquence.",
        ));
    }
    request.knowledge_sequence = Some(k);
    let mut warnings = Vec::new();
    let mut source_ids = BTreeSet::new();
    let mut result_positions = Vec::new();
    let mut result_totals = None;
    let mut result_members = Vec::new();
    let mut result_history = Vec::new();
    if request.context == "company" {
        if financial.is_some() || !history.is_empty() || !members.is_empty() || variation.is_some()
        {
            return Err(integrity(
                "Une vue de pilotage société ne doit pas simuler un bilan familial.",
            ));
        }
        let c = company
            .as_ref()
            .ok_or_else(|| integrity("Le rapport de société est absent."))?;
        if Some(&c.company_id) != request.entity_id.as_ref()
            || c.date != request.date
            || c.knowledge_sequence != k
        {
            return Err(integrity(
                "Le rapport de société ne correspond pas au contexte.",
            ));
        }
        source_ids.extend(c.source_ids.iter().cloned());
        warnings.extend(c.warnings.iter().map(|w| FinancialWarning {
            code: format!("company_{}", w.code),
            object_id: w.object_id.clone(),
            message: w.message.clone(),
        }));
    } else {
        let dates = dashboard_dates(&request)?;
        if company.is_some() {
            return Err(integrity(
                "Un rapport de société ne correspond pas au contexte famille ou membre.",
            ));
        }
        let f = financial
            .as_ref()
            .ok_or_else(|| integrity("Le bilan source est absent."))?;
        check_financial(
            f,
            &request.context,
            request.entity_id.as_deref(),
            &request.date,
            k,
        )?;
        result_positions = positions(sources, f, &request.filters)?;
        result_totals = Some(totals(&result_positions, f)?);
        warnings.extend(financial_warnings(sources, f));
        source_ids.extend(f.source_ids.iter().cloned());
        if history.iter().map(|r| r.date.as_str()).collect::<Vec<_>>()
            != dates.iter().map(String::as_str).collect::<Vec<_>>()
        {
            return Err(integrity(
                "La série historique ne contient pas les observations demandées.",
            ));
        }
        for h in history {
            check_financial(
                &h,
                &request.context,
                request.entity_id.as_deref(),
                &h.date,
                k,
            )?;
            let p = positions(sources, &h, &request.filters)?;
            let point_warnings = financial_warnings(sources, &h);
            source_ids.extend(h.source_ids.iter().cloned());
            result_history.push(DashboardPoint {
                date: h.date.clone(),
                status: h.status.clone(),
                totals: totals(&p, &h)?,
                source_ids: h.source_ids,
                warnings: point_warnings,
            });
        }
        let registry = apply_profile_periods(sources.registry, &request.date)?;
        let mut seen_members = BTreeSet::new();
        for (id, f) in members {
            if request.context != "family" || !seen_members.insert(id.clone()) {
                return Err(integrity("La répartition par membre est incohérente."));
            }
            let member = registry
                .entities
                .iter()
                .find(|e| e.id == id && e.kind == "person")
                .ok_or_else(|| integrity("Un membre de la répartition est inconnu."))?;
            if !registry.memberships.iter().any(|m| {
                m.person_id == id && applies(&m.valid_from, m.valid_to.as_deref(), &request.date)
            }) {
                return Err(integrity(
                    "Un membre de la répartition est hors du périmètre familial.",
                ));
            }
            check_financial(&f, "person", Some(&id), &request.date, k)?;
            let p = positions(sources, &f, &request.filters)?;
            source_ids.extend(f.source_ids.iter().cloned());
            result_members.push(DashboardMember {
                entity_id: id,
                label: member.display_name.clone(),
                totals: totals(&p, &f)?,
            });
        }
        if let Some(v) = &variation {
            if v.from_date != request.from_date
                || v.to_date != request.date
                || v.knowledge_sequence != k
                || v.opening.context != request.context
                || v.opening.person_id != request.entity_id
                || v.closing.context != request.context
                || v.closing.person_id != request.entity_id
                || v.opening.date != request.from_date
                || v.closing.date != request.date
                || v.opening.knowledge_sequence != k
                || v.closing.knowledge_sequence != k
            {
                return Err(integrity(
                    "Le pont de variation ne correspond pas au contexte.",
                ));
            }
            source_ids.extend(
                v.opening
                    .contributions
                    .iter()
                    .chain(&v.closing.contributions)
                    .flat_map(|p| p.source_ids.iter().cloned()),
            );
            for step in &v.steps {
                source_ids.extend(step.source_ids.iter().cloned());
            }
        }
    }
    if let Some(r) = &rental {
        if r.knowledge_sequence != k
            || r.date != request.date
            || r.from_date != request.from_date
            || r.to_date != request.date
        {
            return Err(integrity(
                "Le rapport locatif ne correspond pas à la période du tableau de bord.",
            ));
        }
        source_ids.extend(r.source_ids.iter().cloned());
        warnings.extend(r.warnings.iter().map(|w| FinancialWarning {
            code: format!("rental_{}", w.code),
            object_id: w.object_id.clone(),
            message: w.message.clone(),
        }));
    }
    let period_change = match (result_history.first(), result_history.last()) {
        (Some(first), Some(last)) => {
            let change =
                ratio(&last.totals.exact_known_net)? - ratio(&first.totals.exact_known_net)?;
            Some(DashboardChange {
                from_date: first.date.clone(),
                to_date: last.date.clone(),
                known_net_change_eur: format_eur(&change),
                exact_known_net_change: fraction_of(&change),
                net_change_eur: (first.totals.net_eur.is_some() && last.totals.net_eur.is_some())
                    .then(|| format_eur(&change)),
            })
        }
        _ => None,
    };
    warnings.sort_by(|a, b| {
        (&a.code, &a.object_id, &a.message).cmp(&(&b.code, &b.object_id, &b.message))
    });
    warnings
        .dedup_by(|a, b| a.code == b.code && a.object_id == b.object_id && a.message == b.message);
    let by_category = buckets(&result_positions, "category")?;
    let by_country = buckets(&result_positions, "country")?;
    let by_currency = buckets(&result_positions, "currency")?;
    Ok(DashboardReport {
        filters_applied: filtered(&request.filters),
        request,
        knowledge_sequence: k,
        engine_version: DASHBOARD_ENGINE_VERSION.into(),
        financial,
        company,
        rental,
        totals: result_totals,
        positions: result_positions,
        by_category,
        by_country,
        by_currency,
        members: result_members,
        history: result_history,
        period_change,
        variation,
        warnings,
        source_ids: source_ids.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn f(n: &str, d: &str) -> Fraction {
        Fraction {
            num: n.into(),
            den: d.into(),
        }
    }
    fn request(from: &str, to: &str, interval: &str) -> DashboardRequest {
        DashboardRequest {
            context: "family".into(),
            entity_id: None,
            date: to.into(),
            from_date: from.into(),
            interval: interval.into(),
            knowledge_sequence: Some(7),
            filters: DashboardFilters::default(),
        }
    }
    fn position(
        id: &str,
        kind: &str,
        amount: Option<Fraction>,
        country: &str,
        owner: Option<&str>,
    ) -> FinancialPosition {
        FinancialPosition {
            object_id: id.into(),
            label: format!("Position {id}"),
            kind: kind.into(),
            entity_id: owner.map(str::to_owned),
            country: Some(country.into()),
            currency: "EUR".into(),
            amount_original: None,
            exact_original: None,
            weight: Some(f("1", "1")),
            amount_eur: amount.as_ref().map(|v| format_eur(&ratio(v).unwrap())),
            exact_eur: amount,
            source_ids: vec![format!("source-{id}")],
            conversion: None,
        }
    }
    fn financial(
        positions: Vec<FinancialPosition>,
        eliminated: &str,
        assets: &str,
        liabilities: &str,
        net: &str,
    ) -> FinancialReport {
        let e = eliminated.parse::<BigInt>().unwrap();
        FinancialReport {
            context: "family".into(),
            person_id: None,
            date: "2026-03-31".into(),
            knowledge_sequence: 7,
            status: "complete".into(),
            coverage_confirmed: true,
            assets_eur: Some(assets.into()),
            liabilities_eur: Some(liabilities.into()),
            net_eur: Some(net.into()),
            exact_assets: None,
            exact_liabilities: None,
            exact_net: None,
            known_assets_eur: assets.into(),
            known_liabilities_eur: liabilities.into(),
            known_net_eur: net.into(),
            positions,
            eliminations: if e.is_zero() {
                Vec::new()
            } else {
                vec![FinancialElimination {
                    object_id: "loan".into(),
                    label: "Prêt interne".into(),
                    creditor_before_eur: "1.00".into(),
                    debtor_before_eur: "2.00".into(),
                    eliminated_eur: "0.50".into(),
                    creditor_after_eur: "0.50".into(),
                    debtor_after_eur: "1.50".into(),
                    exact_eliminated: f("1", "2"),
                    source_ids: vec!["elimination-source".into()],
                }]
            },
            source_ids: vec!["scope-source".into()],
            warnings: Vec::new(),
        }
    }
    fn basic_report() -> FinancialReport {
        financial(
            vec![
                position(
                    "loan",
                    "loan_receivable",
                    Some(f("2", "3")),
                    "FR",
                    Some("alice"),
                ),
                position(
                    "loan",
                    "loan_receivable",
                    Some(f("1", "3")),
                    "CH",
                    Some("benoit"),
                ),
                position(
                    "loan",
                    "loan_debt",
                    Some(f("2", "1")),
                    "TN",
                    Some("company"),
                ),
                position("guarantee", "guarantee", Some(f("99999", "1")), "FR", None),
            ],
            "1",
            "0.50",
            "1.50",
            "-1.00",
        )
    }
    fn sources<'a>(
        r: &'a RegistrySnapshot,
        p: &'a PropertySnapshot,
        t: &'a TreasurySnapshot,
        m: &'a MetalSnapshot,
    ) -> DashboardSources<'a> {
        DashboardSources {
            registry: r,
            properties: p,
            treasury: t,
            metals: m,
        }
    }
    fn snapshots() -> (
        RegistrySnapshot,
        PropertySnapshot,
        TreasurySnapshot,
        MetalSnapshot,
    ) {
        (
            RegistrySnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            PropertySnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            TreasurySnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
            MetalSnapshot {
                knowledge_sequence: 7,
                ..Default::default()
            },
        )
    }
    fn build(q: DashboardRequest, report: FinancialReport) -> Result<DashboardReport> {
        let (r, p, t, m) = snapshots();
        let mut history = Vec::new();
        for day in dashboard_dates(&q)? {
            let mut h = report.clone();
            h.date = day;
            history.push(h);
        }
        build_dashboard(
            sources(&r, &p, &t, &m),
            q,
            Some(report),
            None,
            None,
            Vec::new(),
            history,
            None,
        )
    }
    fn entity(id: &str, name: &str) -> Entity {
        Entity {
            id: id.into(),
            revision_id: format!("rev-{id}"),
            kind: "person".into(),
            display_name: name.into(),
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

    #[test]
    fn calendar_contains_exact_endpoints_and_civil_month_ends_including_leap_day() {
        let q = request("2024-01-15", "2024-03-10", "monthly");
        assert_eq!(
            dashboard_dates(&q).unwrap(),
            ["2024-01-15", "2024-01-31", "2024-02-29", "2024-03-10"]
        );
        let q = request("2024-01-31", "2024-03-31", "monthly");
        assert_eq!(
            dashboard_dates(&q).unwrap(),
            ["2024-01-31", "2024-02-29", "2024-03-31"]
        );
    }
    #[test]
    fn calendar_annual_and_same_day_do_not_invent_observations() {
        assert_eq!(
            dashboard_dates(&request("2023-07-01", "2025-02-01", "annual")).unwrap(),
            ["2023-07-01", "2023-12-31", "2024-12-31", "2025-02-01"]
        );
        assert_eq!(
            dashboard_dates(&request("2026-03-31", "2026-03-31", "monthly")).unwrap(),
            ["2026-03-31"]
        );
        assert_eq!(
            dashboard_dates(&request("9999-12-01", "9999-12-31", "monthly")).unwrap(),
            ["9999-12-01", "9999-12-31"]
        );
    }
    #[test]
    fn invalid_dates_contexts_filters_and_observation_budgets_are_rejected() {
        for q in [
            request("2026-02-29", "2026-03-31", "monthly"),
            request("2026-04-01", "2026-03-31", "monthly"),
            request("2026-01-01", "2026-03-31", "weekly"),
            request("2000-01-01", "2026-03-31", "monthly"),
        ] {
            assert!(dashboard_dates(&q).is_err());
        }
        let mut q = request("2026-03-31", "2026-03-31", "monthly");
        q.filters.query = "text\0".into();
        assert!(validate_dashboard_request(&q).is_err());
        q.filters = DashboardFilters::default();
        q.context = "company".into();
        assert!(validate_dashboard_request(&q).is_err());
        q.entity_id = Some("societe".into());
        assert!(validate_dashboard_request(&q).is_ok());
        q.filters.country = Some("FR".into());
        assert!(validate_dashboard_request(&q).is_err());
    }
    #[test]
    fn sixty_one_observations_are_allowed_and_sixty_two_are_rejected() {
        assert_eq!(
            dashboard_dates(&request("2020-01-01", "2024-12-31", "monthly"))
                .unwrap()
                .len(),
            61
        );
        assert!(dashboard_dates(&request("2020-01-01", "2025-01-01", "monthly")).is_err());
    }
    #[test]
    fn eliminated_faces_reconcile_exactly_and_contingent_guarantee_is_not_capitalized() {
        let q = request("2026-03-31", "2026-03-31", "monthly");
        let result = build(q, basic_report()).unwrap();
        let totals = result.totals.unwrap();
        assert_eq!(totals.exact_known_assets, f("1", "2"));
        assert_eq!(totals.exact_known_liabilities, f("3", "2"));
        assert_eq!(totals.position_count, 3);
        assert_eq!(
            result
                .positions
                .iter()
                .find(|p| p.position.entity_id.as_deref() == Some("alice"))
                .unwrap()
                .exact_eur,
            Some(f("1", "3"))
        );
        assert_eq!(
            result
                .positions
                .iter()
                .find(|p| p.position.entity_id.as_deref() == Some("benoit"))
                .unwrap()
                .exact_eur,
            Some(f("1", "6"))
        );
        for groups in [&result.by_category, &result.by_country, &result.by_currency] {
            let sum = groups
                .iter()
                .map(|b| ratio(&b.exact_assets).unwrap())
                .sum::<BigRational>();
            assert_eq!(sum, BigRational::new(1.into(), 2.into()));
        }
        assert!(
            result
                .positions
                .iter()
                .find(|p| p.side == "contingent")
                .unwrap()
                .exact_eliminated
                .num
                == "0"
        );
    }
    #[test]
    fn filters_run_after_elimination_and_preserve_original_source_report() {
        let mut q = request("2026-03-31", "2026-03-31", "monthly");
        q.filters.country = Some("FR".into());
        let result = build(q, basic_report()).unwrap();
        assert_eq!(result.totals.unwrap().exact_known_assets, f("1", "3"));
        assert_eq!(result.financial.unwrap().known_assets_eur, "0.50");
        assert_eq!(result.history[0].totals.exact_known_assets, f("1", "3"));
        let p = result
            .positions
            .iter()
            .find(|p| p.position.kind != "guarantee")
            .unwrap();
        assert_eq!(p.position.exact_eur, Some(f("2", "3")));
        assert_eq!(p.exact_eliminated, f("1", "3"));
        assert!(p.position.source_ids.contains(&"elimination-source".into()));
    }
    #[test]
    fn incomplete_sources_remain_incomplete_even_when_filter_hides_unknown_position() {
        let mut r = basic_report();
        r.positions
            .push(position("unknown", "asset", None, "CH", None));
        r.status = "partial".into();
        r.assets_eur = None;
        r.net_eur = None;
        r.warnings.push(FinancialWarning {
            code: "missing_valuation".into(),
            object_id: "unknown".into(),
            message: "Valeur inconnue".into(),
        });
        let mut q = request("2026-03-31", "2026-03-31", "monthly");
        q.filters.country = Some("TN".into());
        let result = build(q, r).unwrap();
        let t = result.totals.unwrap();
        assert_eq!(t.known_assets_eur, "0.00");
        assert!(t.assets_eur.is_none() && t.net_eur.is_none());
        assert!(result.warnings.iter().any(|w| w.object_id == "unknown"));
        assert!(result.period_change.unwrap().net_change_eur.is_none());
    }
    #[test]
    fn unknown_zero_and_negative_values_stay_distinct_and_exact() {
        let report = financial(
            vec![
                position(
                    "negative-equity",
                    "global_equity",
                    Some(f("-1", "3")),
                    "FR",
                    None,
                ),
                position("zero", "asset", Some(f("0", "1")), "FR", None),
                position("unknown", "asset", None, "FR", None),
            ],
            "0",
            "-0.33",
            "0.00",
            "-0.33",
        );
        let mut report = report;
        report.assets_eur = None;
        report.net_eur = None;
        let result = build(request("2026-03-31", "2026-03-31", "monthly"), report).unwrap();
        let t = result.totals.unwrap();
        assert_eq!(t.exact_known_assets, f("-1", "3"));
        assert_eq!(t.unknown_count, 1);
        assert_eq!(
            result
                .positions
                .iter()
                .find(|p| p.position.object_id == "zero")
                .unwrap()
                .amount_eur
                .as_deref(),
            Some("0.00")
        );
        assert!(result
            .positions
            .iter()
            .find(|p| p.position.object_id == "unknown")
            .unwrap()
            .amount_eur
            .is_none());
    }
    #[test]
    fn long_precise_values_are_not_rounded_before_grouping() {
        let original = f("123456789012345678901234", "10000");
        let report = financial(
            vec![position(
                "large",
                "asset",
                Some(original.clone()),
                "FR",
                None,
            )],
            "0",
            "12345678901234567890.12",
            "0.00",
            "12345678901234567890.12",
        );
        let result = build(request("2026-03-31", "2026-03-31", "monthly"), report).unwrap();
        assert_eq!(
            ratio(&result.by_country[0].exact_assets).unwrap(),
            ratio(&original).unwrap()
        );
        assert_eq!(
            result.by_country[0].known_assets_eur,
            "12345678901234567890.12"
        );
    }
    #[test]
    fn invalid_or_mismatched_eliminations_fail_instead_of_creating_unattributed_totals() {
        let q = request("2026-03-31", "2026-03-31", "monthly");
        let mut r = basic_report();
        r.eliminations[0].object_id = "another-loan".into();
        assert!(build(q.clone(), r).is_err());
        let mut r = basic_report();
        r.eliminations[0].exact_eliminated = f("3", "1");
        assert!(build(q.clone(), r).is_err());
        let mut r = basic_report();
        r.known_assets_eur = "999.00".into();
        assert!(build(q, r).is_err());
    }
    #[test]
    fn missing_historical_points_or_mixed_knowledge_fail_explicitly() {
        let (r, p, t, m) = snapshots();
        let q = request("2026-01-01", "2026-03-31", "monthly");
        assert!(build_dashboard(
            sources(&r, &p, &t, &m),
            q.clone(),
            Some(basic_report()),
            None,
            None,
            Vec::new(),
            vec![basic_report()],
            None
        )
        .is_err());
        let mut t = t;
        t.knowledge_sequence = 8;
        assert!(build_dashboard(
            sources(&r, &p, &t, &m),
            q,
            Some(basic_report()),
            None,
            None,
            Vec::new(),
            Vec::new(),
            None
        )
        .is_err());
    }
    #[test]
    fn historical_owner_name_search_uses_profile_at_economic_date() {
        let (mut r, p, t, m) = snapshots();
        let current = entity("alice", "Nom actuel");
        let mut old = current.clone();
        old.revision_id = "old-profile".into();
        old.display_name = "Ancien nom daté".into();
        r.entities.push(current);
        r.profile_revisions.push(old);
        r.profile_periods.push(ProfilePeriod {
            id: "period".into(),
            revision_id: "period-rev".into(),
            target_kind: "entity".into(),
            target_id: "alice".into(),
            profile_revision_id: "old-profile".into(),
            valid_from: "2020-01-01".into(),
            valid_to: Some("2027-01-01".into()),
        });
        let report = financial(
            vec![position(
                "asset",
                "asset",
                Some(f("10", "1")),
                "FR",
                Some("alice"),
            )],
            "0",
            "10.00",
            "0.00",
            "10.00",
        );
        let mut q = request("2026-03-31", "2026-03-31", "monthly");
        q.filters.query = "ancien nom".into();
        let result = build_dashboard(
            sources(&r, &p, &t, &m),
            q,
            Some(report.clone()),
            None,
            None,
            Vec::new(),
            vec![report],
            None,
        )
        .unwrap();
        assert_eq!(result.positions.len(), 1);
    }
    #[test]
    fn known_net_change_is_an_exact_difference_and_stays_unknown_for_partial_endpoints() {
        let (r, p, t, m) = snapshots();
        let q = request("2026-02-28", "2026-03-31", "monthly");
        let mut opening = financial(
            vec![position("asset", "asset", Some(f("1", "3")), "FR", None)],
            "0",
            "0.33",
            "0.00",
            "0.33",
        );
        opening.date = "2026-02-28".into();
        opening.net_eur = None;
        let closing = financial(
            vec![position("asset", "asset", Some(f("2", "3")), "FR", None)],
            "0",
            "0.67",
            "0.00",
            "0.67",
        );
        let result = build_dashboard(
            sources(&r, &p, &t, &m),
            q,
            Some(closing.clone()),
            None,
            None,
            Vec::new(),
            vec![opening, closing],
            None,
        )
        .unwrap();
        let change = result.period_change.unwrap();
        assert_eq!(change.exact_known_net_change, f("1", "3"));
        assert_eq!(change.known_net_change_eur, "0.33");
        assert!(change.net_change_eur.is_none());
    }
    #[test]
    fn selected_valuation_is_warned_after_365_days_without_changing_the_amount() {
        let (mut r, p, t, m) = snapshots();
        r.assets.push(Asset {
            id: "asset".into(),
            revision_id: "asset-rev".into(),
            name: "Bien".into(),
            country: "FR".into(),
            category: "autre".into(),
            notes: String::new(),
            archived: false,
        });
        for (id, on) in [
            ("v365", "2025-03-31"),
            ("v366", "2025-03-30"),
            ("future", "2026-04-01"),
        ] {
            r.valuations.push(Valuation {
                id: id.into(),
                revision_id: format!("rev-{id}"),
                target_kind: "asset".into(),
                target_id: "asset".into(),
                amount: "10".into(),
                currency: "EUR".into(),
                effective_on: on.into(),
                source: "Source vérifiée".into(),
            });
        }
        for (selected, count) in [("v365", 0), ("v366", 1), ("future", 0)] {
            let mut asset = position("asset", "asset", Some(f("10", "1")), "FR", None);
            // Include unrelated valuation object IDs; only the exact revision
            // used by the source report authorizes an age observation.
            asset
                .source_ids
                .extend([format!("rev-{selected}"), "v366".into()]);
            let report = financial(vec![asset], "0", "10.00", "0.00", "10.00");
            let q = request("2026-03-31", "2026-03-31", "monthly");
            let result = build_dashboard(
                sources(&r, &p, &t, &m),
                q,
                Some(report.clone()),
                None,
                None,
                Vec::new(),
                vec![report],
                None,
            )
            .unwrap();
            assert_eq!(
                result
                    .warnings
                    .iter()
                    .filter(|w| w.code == "stale_valuation")
                    .count(),
                count
            );
            assert_eq!(
                result.history[0]
                    .warnings
                    .iter()
                    .filter(|w| w.code == "stale_valuation")
                    .count(),
                count
            );
            assert_eq!(result.totals.unwrap().net_eur.as_deref(), Some("10.00"));
            if count == 1 {
                assert!(result
                    .warnings
                    .iter()
                    .any(|w| w.message.contains("366 jours") && w.message.contains("rev-v366")));
            }
        }
    }
}
