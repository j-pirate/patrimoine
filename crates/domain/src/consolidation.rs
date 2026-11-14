//! Pure exact consolidation over an explicitly versioned registry snapshot.
use crate::exact::{format_eur, fraction_of, parse_decimal, parse_fraction};
use crate::fx::{convert_currency_unchecked, converted_rational};
use crate::profiles::profile_is_dated;
use crate::validation::company_order;
use crate::*;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConsolidationRequest {
    pub context: String,
    pub person_id: Option<String>,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub object_id: String,
    pub label: String,
    pub kind: String,
    pub country: Option<String>,
    pub category: String,
    pub weight: Fraction,
    pub amount_original: String,
    pub currency: String,
    pub valuation_id: Option<String>,
    pub valuation_revision_id: Option<String>,
    pub valuation_date: Option<String>,
    pub amount_eur: String,
    pub exact_eur: Fraction,
    pub source_ids: Vec<String>,
    #[serde(default)]
    pub conversion: Option<FxResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub context: String,
    pub person_id: Option<String>,
    pub date: String,
    pub knowledge_sequence: u64,
    pub engine_version: String,
    pub status: String,
    pub gross_eur: String,
    pub debt_eur: String,
    pub net_eur: String,
    pub exact_gross: Fraction,
    pub exact_debt: Fraction,
    pub exact_net: Fraction,
    pub global_equity_eur: String,
    pub gross_is_underlying_complete: bool,
    pub contributions: Vec<Contribution>,
    pub warnings: Vec<ConsolidationWarning>,
}

/// Phase 3 domain contract only. The persisted debt module arrives in phase 6.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebtParty {
    pub entity_id: String,
    pub economic_share: Option<Fraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoanPosition {
    pub id: String,
    pub principal: String,
    pub currency: String,
    pub creditors: Vec<DebtParty>,
    pub debtors: Vec<DebtParty>,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub contingent: bool,
}

#[derive(Clone, Default)]
struct Weight {
    known: BigRational,
    unknown: bool,
    sources: BTreeSet<String>,
}

/// Analytical income rights are independent from the stock valuation method.
/// Looking through a company here never declares a cash distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicWeight {
    pub weight: Fraction,
    pub unknown: bool,
    pub source_ids: Vec<String>,
}

fn income_weight<'a>(
    rows: impl Iterator<Item = (&'a str, &'a str, &'a str, &'a Option<Fraction>)>,
    weights: &BTreeMap<String, Weight>,
    has_scope: bool,
) -> Result<Weight> {
    let mut out = Weight::default();
    let mut total = BigRational::zero();
    for (owner, id, revision, share) in rows {
        let parent = weights.get(owner).cloned().unwrap_or_default();
        if let Some(share) = share {
            let share = parse_fraction(share)?;
            total += &share;
            out.known += &parent.known * &share;
            out.unknown |= parent.unknown && !share.is_zero();
            if !share.is_zero() && (!parent.known.is_zero() || parent.unknown) {
                out.sources.extend(parent.sources);
                out.sources.extend([id.to_owned(), revision.to_owned()]);
            }
        } else if !parent.known.is_zero() || parent.unknown {
            out.unknown = true;
            out.sources.extend(parent.sources);
            out.sources.extend([id.to_owned(), revision.to_owned()]);
        }
        check_size(&out.known)?;
    }
    out.unknown |= has_scope && total < BigRational::one();
    Ok(out)
}

/// Rights at a civil date, without requiring an asset estimate or applying
/// stock global-equity barriers. `property` observes the whole asset's income.
pub fn economic_asset_weight(
    s: &RegistrySnapshot,
    asset_id: &str,
    context: &str,
    person_id: Option<&str>,
    date: &str,
) -> Result<EconomicWeight> {
    validate_registry(s)?;
    validate_date(date)?;
    let asset = s
        .assets
        .iter()
        .find(|a| a.id == asset_id)
        .ok_or_else(|| AppError::new("validation", "L’actif du revenu est inconnu."))?;
    if context == "property" && person_id.is_none() {
        return Ok(EconomicWeight {
            weight: fraction_of(&BigRational::one()),
            unknown: false,
            source_ids: vec![asset.id.clone(), asset.revision_id.clone()],
        });
    }
    if !((context == "family" && person_id.is_none())
        || (context == "person"
            && person_id
                .is_some_and(|id| s.entities.iter().any(|e| e.id == id && e.kind == "person")))
        || (context == "company"
            && person_id
                .is_some_and(|id| s.entities.iter().any(|e| e.id == id && e.kind == "company"))))
    {
        return Err(AppError::new(
            "validation",
            "Le périmètre d’attribution du revenu est invalide.",
        ));
    }
    let request = ConsolidationRequest {
        context: context.into(),
        person_id: person_id.map(str::to_owned),
        date: date.into(),
        knowledge_sequence: Some(s.knowledge_sequence),
    };
    let mut weights = root_weights(s, &request);
    if context == "company" {
        let company = s
            .entities
            .iter()
            .find(|e| Some(e.id.as_str()) == person_id)
            .expect("validated company context");
        weights.insert(
            company.id.clone(),
            Weight {
                known: BigRational::one(),
                unknown: false,
                sources: BTreeSet::from([company.id.clone(), company.revision_id.clone()]),
            },
        );
    }
    if context == "family" {
        for membership in s
            .memberships
            .iter()
            .filter(|m| applies(&m.valid_from, m.valid_to.as_deref(), date))
        {
            if let Some(weight) = weights.get_mut(&membership.person_id) {
                weight
                    .sources
                    .extend([membership.id.clone(), membership.revision_id.clone()]);
            }
        }
    }
    let has_scope = weights.values().any(|w| !w.known.is_zero());
    for company in company_order(s, date)? {
        if context == "company" && Some(company.as_str()) == person_id {
            continue;
        }
        let incoming = s
            .participations
            .iter()
            .filter(|p| {
                p.company_id == company && applies(&p.valid_from, p.valid_to.as_deref(), date)
            })
            .map(|p| {
                (
                    p.owner_entity_id.as_str(),
                    p.id.as_str(),
                    p.revision_id.as_str(),
                    &p.economic_share,
                )
            });
        let weight = income_weight(incoming, &weights, has_scope)?;
        weights.insert(company, weight);
    }
    let incoming = s
        .ownerships
        .iter()
        .filter(|o| o.asset_id == asset_id && applies(&o.valid_from, o.valid_to.as_deref(), date))
        .map(|o| {
            (
                o.owner_entity_id.as_str(),
                o.id.as_str(),
                o.revision_id.as_str(),
                &o.economic_share,
            )
        });
    let weight = income_weight(incoming, &weights, has_scope)?;
    Ok(EconomicWeight {
        weight: fraction_of(&weight.known),
        unknown: weight.unknown,
        source_ids: weight.sources.into_iter().collect(),
    })
}

fn warn(warnings: &mut Vec<ConsolidationWarning>, code: &str, object: &str, message: &str) {
    if !warnings
        .iter()
        .any(|w| w.code == code && w.object_id == object)
    {
        warnings.push(ConsolidationWarning {
            code: code.into(),
            object_id: object.into(),
            message: message.into(),
        });
    }
}

fn check_size(value: &BigRational) -> Result<()> {
    if value.numer().bits().saturating_add(value.denom().bits()) > 131_072 {
        return Err(AppError::new("capacity","Le calcul exact dépasse le budget de précision du socle. Aucun montant approximatif n’a été substitué."));
    }
    Ok(())
}

fn policy<'a>(s: &'a RegistrySnapshot, id: &str, date: &str) -> Option<&'a CompanyPolicy> {
    s.company_policies
        .iter()
        .find(|p| p.company_id == id && applies(&p.valid_from, p.valid_to.as_deref(), date))
}

fn root_weights(s: &RegistrySnapshot, r: &ConsolidationRequest) -> BTreeMap<String, Weight> {
    s.entities
        .iter()
        .filter(|e| e.kind == "person")
        .map(|e| {
            let selected = if r.context == "family" {
                s.memberships.iter().any(|m| {
                    m.person_id == e.id && applies(&m.valid_from, m.valid_to.as_deref(), &r.date)
                })
            } else {
                r.person_id.as_deref() == Some(&e.id)
            };
            let sources = if selected {
                BTreeSet::from([e.id.clone(), e.revision_id.clone()])
            } else {
                BTreeSet::new()
            };
            (
                e.id.clone(),
                Weight {
                    known: if selected {
                        BigRational::one()
                    } else {
                        BigRational::zero()
                    },
                    unknown: false,
                    sources,
                },
            )
        })
        .collect()
}

/// One target receives all incoming paths. Missing fractions and an unassigned
/// remainder are tracked separately from its known contribution.
fn target_weight<'a>(
    rows: impl Iterator<Item = (&'a str, &'a str, &'a str, &'a Option<Fraction>)>,
    weights: &BTreeMap<String, Weight>,
    s: &RegistrySnapshot,
    date: &str,
    has_scope: bool,
) -> Result<Weight> {
    let mut out = Weight::default();
    let mut total = BigRational::zero();
    for (owner, id, revision, share) in rows {
        let mut parent = weights.get(owner).cloned().unwrap_or_default();
        if s.entities
            .iter()
            .any(|e| e.id == owner && e.kind == "company")
            && !policy(s, owner, date).is_some_and(|p| p.mode == "detailed")
        {
            // A global-equity or unspecified method is a boundary on this path.
            parent = Weight::default();
        }
        if let Some(share) = share {
            let fraction = parse_fraction(share)?;
            total += &fraction;
            out.known += &parent.known * &fraction;
            out.unknown |= parent.unknown && !fraction.is_zero();
            if !fraction.is_zero() && (!parent.known.is_zero() || parent.unknown) {
                out.sources.extend(parent.sources);
                out.sources.extend([id.to_owned(), revision.to_owned()]);
            }
        } else if !parent.known.is_zero() || parent.unknown {
            out.unknown = true;
            out.sources.extend(parent.sources);
            out.sources.extend([id.to_owned(), revision.to_owned()]);
        }
        check_size(&out.known)?;
    }
    if has_scope && total < BigRational::one() {
        out.unknown = true;
    }
    Ok(out)
}

fn weights(s: &RegistrySnapshot, r: &ConsolidationRequest) -> Result<BTreeMap<String, Weight>> {
    let mut out = root_weights(s, r);
    let has_scope = out.values().any(|w| !w.known.is_zero());
    for id in company_order(s, &r.date)? {
        let incoming = s
            .participations
            .iter()
            .filter(|p| {
                p.company_id == id && applies(&p.valid_from, p.valid_to.as_deref(), &r.date)
            })
            .map(|p| {
                (
                    p.owner_entity_id.as_str(),
                    p.id.as_str(),
                    p.revision_id.as_str(),
                    &p.economic_share,
                )
            });
        let weight = target_weight(incoming, &out, s, &r.date, has_scope)?;
        out.insert(id, weight);
    }
    Ok(out)
}

fn latest_value<'a>(
    s: &'a RegistrySnapshot,
    kind: &str,
    id: &str,
    date: &str,
) -> Option<&'a Valuation> {
    s.valuations
        .iter()
        .filter(|v| v.target_kind == kind && v.target_id == id && v.effective_on.as_str() <= date)
        .max_by(|a, b| a.effective_on.cmp(&b.effective_on))
}

struct Position<'a> {
    id: &'a str,
    label: &'a str,
    kind: &'a str,
    target_kind: &'a str,
    country: Option<String>,
    category: String,
    weight: Weight,
}

fn value_position(
    s: &RegistrySnapshot,
    date: &str,
    valuation_date: &str,
    fx_date: &str,
    position: Position<'_>,
    contributions: &mut Vec<Contribution>,
    warnings: &mut Vec<ConsolidationWarning>,
) -> Result<Option<BigRational>> {
    let Position {
        id,
        label,
        kind,
        target_kind,
        country,
        category,
        weight,
    } = position;
    if weight.unknown {
        warn(warnings,"unknown_economic_rights",id,"Les droits économiques sont incomplets ou inconnus. Seule la fraction connue peut être attribuée.");
    }
    if weight.known.is_zero() {
        return Ok(None);
    }
    let Some(value) = latest_value(s, target_kind, id, valuation_date) else {
        warn(warnings,"missing_valuation",id,"Aucune valorisation applicable à cette date n’est connue. Aucune valeur future n’a été utilisée.");
        return Ok(None);
    };
    let conversion = convert_currency_unchecked(
        s,
        &FxQuery {
            amount: value.amount.clone(),
            from_currency: value.currency.clone(),
            to_currency: "EUR".into(),
            date: fx_date.into(),
            knowledge_sequence: Some(s.knowledge_sequence),
        },
    )?;
    for warning in &conversion.warnings {
        warn(warnings, &warning.code, id, &warning.message);
    }
    let Some(converted) = converted_rational(&conversion) else {
        return Ok(None);
    };
    let metadata_kind = if target_kind == "company" {
        "entity"
    } else {
        "asset"
    };
    let dated_profile = profile_is_dated(s, metadata_kind, id, date);
    if !dated_profile {
        warn(warnings,"undated_profile",id,&format!("Au {date}, aucune période ne date le profil descriptif. Le libellé reflète les connaissances de la séquence {} ; le pays historique n’est pas attribué par hypothèse.",s.knowledge_sequence));
    }
    let exact = &weight.known * converted;
    check_size(&exact)?;
    let mut sources = weight.sources;
    sources.extend([id.to_owned(), value.id.clone(), value.revision_id.clone()]);
    for leg in &conversion.legs {
        sources.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
    }
    for period in s.profile_periods.iter().filter(|p| {
        p.target_kind == metadata_kind
            && p.target_id == id
            && applies(&p.valid_from, p.valid_to.as_deref(), date)
    }) {
        sources.extend([
            period.id.clone(),
            period.revision_id.clone(),
            period.profile_revision_id.clone(),
        ]);
    }
    contributions.push(Contribution {
        object_id: id.into(),
        label: label.into(),
        kind: kind.into(),
        country: if dated_profile { country } else { None },
        category,
        weight: fraction_of(&weight.known),
        amount_original: value.amount.clone(),
        currency: value.currency.clone(),
        valuation_id: Some(value.id.clone()),
        valuation_revision_id: Some(value.revision_id.clone()),
        valuation_date: Some(value.effective_on.clone()),
        amount_eur: format_eur(&exact),
        exact_eur: fraction_of(&exact),
        source_ids: sources.into_iter().collect(),
        conversion: Some(conversion),
    });
    Ok(Some(exact))
}

/// Current application entry point: exact attribution of the recorded EUR
/// inventory, explicitly incomplete while liabilities are not inventoried.
pub fn consolidate(s: &RegistrySnapshot, r: &ConsolidationRequest) -> Result<ConsolidationReport> {
    consolidate_with_loans(s, r, &[], false)
}

/// Physical lots replace generic estimates without creating synthetic valuations.
pub fn consolidate_with_metals(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    treasury: &TreasurySnapshot,
    metals: &MetalSnapshot,
    request: &ConsolidationRequest,
) -> Result<ConsolidationReport> {
    validate_metals(registry, properties, treasury, metals)?;
    if metals.lots.is_empty() {
        return consolidate(registry, request);
    }
    let metal_assets: BTreeSet<_> = metals.lots.iter().map(|l| l.asset_id.as_str()).collect();
    let mut filtered = registry.clone();
    filtered
        .valuations
        .retain(|v| v.target_kind != "asset" || !metal_assets.contains(v.target_id.as_str()));
    let mut report = consolidate(&filtered, request)?;
    report.warnings.retain(|w| {
        !(w.code == "missing_valuation" && metal_assets.contains(w.object_id.as_str()))
    });
    report.gross_is_underlying_complete = !report
        .contributions
        .iter()
        .any(|c| c.kind == "global_equity")
        && report.warnings.iter().all(|w| {
            [
                "liabilities_not_inventoried",
                "internal_loan_elimination",
                "contingent_guarantee",
                "undated_profile",
            ]
            .contains(&w.code.as_str())
        });
    let profiled = apply_profile_periods(registry, &request.date)?;
    let weights = weights(&profiled, request)?;
    let has_scope = weights.values().any(|w| !w.known.is_zero());
    let mut gross = num_rational::BigRational::new(
        report
            .exact_gross
            .num
            .parse()
            .map_err(|_| AppError::new("validation", "Sous-total métal invalide."))?,
        report
            .exact_gross
            .den
            .parse()
            .map_err(|_| AppError::new("validation", "Sous-total métal invalide."))?,
    );
    for lot in &metals.lots {
        let weight = target_weight(
            profiled
                .ownerships
                .iter()
                .filter(|o| {
                    o.asset_id == lot.asset_id
                        && applies(&o.valid_from, o.valid_to.as_deref(), &request.date)
                })
                .map(|o| {
                    (
                        o.owner_entity_id.as_str(),
                        o.id.as_str(),
                        o.revision_id.as_str(),
                        &o.economic_share,
                    )
                }),
            &weights,
            &profiled,
            &request.date,
            has_scope,
        )?;
        if weight.known.is_zero() {
            continue;
        }
        let value = metal_report(
            registry,
            properties,
            treasury,
            metals,
            &MetalReportRequest {
                lot_id: lot.id.clone(),
                date: request.date.clone(),
                knowledge_sequence: request.knowledge_sequence,
            },
        )?;
        for w in &value.warnings {
            warn(&mut report.warnings, &w.code, &w.object_id, &w.message);
        }
        let Some(fraction) = &value.exact_valuation_eur else {
            report.gross_is_underlying_complete = false;
            warn(&mut report.warnings, "metal_stock_valuation_missing", &lot.asset_id, "La quantité ou les sources de valorisation du lot sont incomplètes. Aucune estimation générique n’a remplacé ce stock.");
            continue;
        };
        let exact = BigRational::new(
            fraction
                .num
                .parse()
                .map_err(|_| AppError::new("validation", "Valeur de métal invalide."))?,
            fraction
                .den
                .parse()
                .map_err(|_| AppError::new("validation", "Valeur de métal invalide."))?,
        ) * &weight.known;
        check_size(&exact)?;
        gross += &exact;
        let mut sources = weight.sources;
        sources.extend(value.source_ids);
        sources.extend([
            lot.asset_id.clone(),
            lot.id.clone(),
            lot.revision_id.clone(),
        ]);
        let amount = if lot.valuation_basis == "intrinsic" {
            value.intrinsic_original
        } else {
            value.resale_original
        };
        report.contributions.push(Contribution {
            object_id: lot.asset_id.clone(),
            label: lot.designation.clone(),
            kind: "metal".into(),
            country: Some(value.country),
            category: lot.metal.clone(),
            weight: fraction_of(&weight.known),
            amount_original: amount.ok_or_else(|| {
                AppError::new(
                    "validation",
                    "La valeur exacte du métal doit conserver son montant d’origine.",
                )
            })?,
            currency: value.currency,
            valuation_id: value.quote.as_ref().map(|q| q.id.clone()),
            valuation_revision_id: value.quote.as_ref().map(|q| q.revision_id.clone()),
            valuation_date: value.quote.as_ref().map(|q| q.effective_on.clone()),
            amount_eur: format_eur(&exact),
            exact_eur: fraction_of(&exact),
            source_ids: sources.into_iter().collect(),
            conversion: None,
        });
    }
    let debt = BigRational::new(
        report
            .exact_debt
            .num
            .parse()
            .map_err(|_| AppError::new("validation", "Passif invalide."))?,
        report
            .exact_debt
            .den
            .parse()
            .map_err(|_| AppError::new("validation", "Passif invalide."))?,
    );
    let net = &gross - debt;
    report.gross_eur = format_eur(&gross);
    report.exact_gross = fraction_of(&gross);
    report.net_eur = format_eur(&net);
    report.exact_net = fraction_of(&net);
    report.engine_version = "registry-metals-3".into();
    report.status = if report.contributions.is_empty() && report.status == "non_calculable" {
        "non_calculable"
    } else {
        "partial"
    }
    .into();
    report.gross_is_underlying_complete &=
        !report.warnings.iter().any(|w| w.code.starts_with("metal_"));
    Ok(report)
}

/// Reproducible counterfactual for the phase-4 subtotal bridge. Each category
/// freezes its own economic date; original IDs and valuation/FX dates survive.
#[allow(clippy::too_many_arguments)]
pub(crate) fn consolidate_counterfactual(
    s: &RegistrySnapshot,
    r: &ConsolidationRequest,
    membership_date: &str,
    method_date: &str,
    rights_date: &str,
    valuation_date: &str,
    fx_date: &str,
) -> Result<ConsolidationReport> {
    validate_registry(s)?;
    for date in [
        membership_date,
        method_date,
        rights_date,
        valuation_date,
        fx_date,
    ] {
        validate_date(date)?;
    }
    let mut frozen = s.clone();
    frozen
        .memberships
        .retain(|p| applies(&p.valid_from, p.valid_to.as_deref(), membership_date));
    frozen
        .company_policies
        .retain(|p| applies(&p.valid_from, p.valid_to.as_deref(), method_date));
    frozen
        .participations
        .retain(|p| applies(&p.valid_from, p.valid_to.as_deref(), rights_date));
    frozen
        .ownerships
        .retain(|p| applies(&p.valid_from, p.valid_to.as_deref(), rights_date));
    for p in &mut frozen.memberships {
        p.valid_from = "0001-01-01".into();
        p.valid_to = None;
    }
    for p in &mut frozen.company_policies {
        p.valid_from = "0001-01-01".into();
        p.valid_to = None;
    }
    for p in &mut frozen.participations {
        p.valid_from = "0001-01-01".into();
        p.valid_to = None;
    }
    for p in &mut frozen.ownerships {
        p.valid_from = "0001-01-01".into();
        p.valid_to = None;
    }
    consolidate_at_dates(&frozen, r, &[], false, valuation_date, fx_date)
}

/// The loan positions supplied here describe a declared in-memory perimeter.
/// Their UI, ledger, historical amortization and persistence are phase 6 work.
pub fn consolidate_with_loans(
    s: &RegistrySnapshot,
    r: &ConsolidationRequest,
    loans: &[LoanPosition],
    liabilities_complete: bool,
) -> Result<ConsolidationReport> {
    consolidate_at_dates(s, r, loans, liabilities_complete, &r.date, &r.date)
}

fn consolidate_at_dates(
    original: &RegistrySnapshot,
    r: &ConsolidationRequest,
    loans: &[LoanPosition],
    liabilities_complete: bool,
    valuation_date: &str,
    fx_date: &str,
) -> Result<ConsolidationReport> {
    let profiled = apply_profile_periods(original, &r.date)?;
    let s = &profiled;
    validate_registry(s)?;
    validate_date(&r.date)?;
    if !["family", "person"].contains(&r.context.as_str())
        || (r.context == "family" && r.person_id.is_some())
        || (r.context == "person"
            && !s
                .entities
                .iter()
                .any(|e| e.kind == "person" && Some(e.id.as_str()) == r.person_id.as_deref()))
        || r.knowledge_sequence
            .is_some_and(|k| k != s.knowledge_sequence)
    {
        return Err(AppError::new(
            "validation",
            "Le contexte ou la borne de connaissance ne correspond pas au registre fourni.",
        ));
    }
    let weights = weights(s, r)?;
    let has_scope = weights.values().any(|w| !w.known.is_zero());
    let mut warnings = Vec::new();
    let mut contributions = Vec::new();
    let mut gross = BigRational::zero();
    let mut debt = BigRational::zero();
    let mut global_equity = BigRational::zero();
    let mut opaque = false;
    for e in s.entities.iter().filter(|e| e.kind == "company") {
        let weight = weights.get(&e.id).cloned().unwrap_or_default();
        if weight.known.is_zero() && !weight.unknown {
            continue;
        }
        if weight.unknown {
            warn(
                &mut warnings,
                "unknown_economic_rights",
                &e.id,
                "La participation économique dans cette société est incomplètement définie.",
            );
        }
        let Some(p) = policy(s, &e.id, &r.date) else {
            warn(&mut warnings,"missing_company_policy",&e.id,"Choisissez une méthode datée : actifs détaillés ou valeur globale nette. Les sous-jacents ne sont pas additionnés par hypothèse.");
            continue;
        };
        if !p.coverage_complete {
            warn(
                &mut warnings,
                "incomplete_company_coverage",
                &e.id,
                "Le périmètre des actifs et passifs de la société n’est pas déclaré complet.",
            );
        }
        if p.mode == "global_equity" {
            opaque = true;
            if latest_value(s, "company", &e.id, valuation_date)
                .is_some_and(|v| parse_decimal(&v.amount).is_ok_and(|a| a.is_negative()))
                && !p.liability_qualified
            {
                warn(&mut warnings,"unqualified_negative_equity",&e.id,"La valeur des parts est négative sans qualification des engagements de l’associé. Elle est une exposition analytique, pas une dette personnelle supposée.");
            }
            let position = Position {
                id: &e.id,
                label: &e.display_name,
                kind: "global_equity",
                target_kind: "company",
                country: e.residence_country.clone(),
                category: "participation globale nette".into(),
                weight,
            };
            if let Some(amount) = value_position(
                s,
                &r.date,
                valuation_date,
                fx_date,
                position,
                &mut contributions,
                &mut warnings,
            )? {
                global_equity += &amount;
                gross += amount;
            }
        }
    }
    for a in &s.assets {
        let incoming = s
            .ownerships
            .iter()
            .filter(|o| {
                o.asset_id == a.id && applies(&o.valid_from, o.valid_to.as_deref(), &r.date)
            })
            .map(|o| {
                (
                    o.owner_entity_id.as_str(),
                    o.id.as_str(),
                    o.revision_id.as_str(),
                    &o.economic_share,
                )
            });
        let weight = target_weight(incoming, &weights, s, &r.date, has_scope)?;
        let position = Position {
            id: &a.id,
            label: &a.name,
            kind: "asset",
            target_kind: "asset",
            country: Some(a.country.clone()),
            category: a.category.clone(),
            weight,
        };
        if let Some(amount) = value_position(
            s,
            &r.date,
            valuation_date,
            fx_date,
            position,
            &mut contributions,
            &mut warnings,
        )? {
            gross += amount;
        }
    }
    let mut loan_ids = BTreeSet::new();
    for loan in loans {
        if loan.id.is_empty() || !loan_ids.insert(&loan.id) {
            return Err(AppError::new(
                "validation",
                "Un contrat de financement est vide ou dupliqué.",
            ));
        }
        validate_interval(&loan.valid_from, loan.valid_to.as_deref())?;
        let principal = parse_decimal(&loan.principal)?;
        if principal.is_negative() {
            return Err(AppError::new(
                "validation",
                "Le capital d’un prêt ne peut pas être négatif.",
            ));
        }
        let (creditor, creditor_unknown) = party_weight(&loan.creditors, &weights, s)?;
        let (debtor, debtor_unknown) = party_weight(&loan.debtors, &weights, s)?;
        if !applies(&loan.valid_from, loan.valid_to.as_deref(), &r.date) {
            continue;
        }
        if loan.contingent {
            warn(&mut warnings,"contingent_guarantee",&loan.id,"Cette garantie est suivie hors passifs certains et n’est pas soustraite du patrimoine.");
            continue;
        }
        let conversion = convert_currency_unchecked(
            s,
            &FxQuery {
                amount: loan.principal.clone(),
                from_currency: loan.currency.clone(),
                to_currency: "EUR".into(),
                date: fx_date.into(),
                knowledge_sequence: Some(s.knowledge_sequence),
            },
        )?;
        for warning in &conversion.warnings {
            warn(&mut warnings, &warning.code, &loan.id, &warning.message);
        }
        let Some(principal) = converted_rational(&conversion) else {
            continue;
        };
        if loan
            .creditors
            .iter()
            .chain(&loan.debtors)
            .any(|p| behind_opaque_boundary(s, &p.entity_id, &r.date))
        {
            warn(&mut warnings,"opaque_loan_coverage",&loan.id,"Un prêt traverse une valorisation globale. Sa couverture doit être rapprochée avant une élimination réciproque.");
            continue;
        }
        if loan.debtors.iter().any(|party| {
            s.entities
                .iter()
                .any(|e| e.id == party.entity_id && e.kind == "company")
                && weights
                    .get(&party.entity_id)
                    .is_some_and(|w| !w.known.is_zero() || w.unknown)
                && !policy(s, &party.entity_id, &r.date).is_some_and(|p| p.liability_qualified)
        }) {
            warn(&mut warnings,"unqualified_company_liability",&loan.id,"La dette de société est une exposition économique analytique. La responsabilité des associés doit être qualifiée avant de présenter un net complet ; aucune dette personnelle n’est présumée.");
        }
        if creditor_unknown || debtor_unknown {
            warn(&mut warnings,"unknown_debt_allocation",&loan.id,"Les allocations créancières ou débitrices sont incomplètes. Aucune répartition fondée sur la propriété d’un bien n’a été supposée.");
            // Keep known sides but do not invent a reciprocal match.
        }
        let eliminated = if creditor_unknown || debtor_unknown {
            BigRational::zero()
        } else {
            creditor.clone().min(debtor.clone())
        };
        let credit_residual = &creditor - &eliminated;
        let debt_residual = &debtor - &eliminated;
        for (kind, weight, total) in [
            ("receivable", credit_residual, &mut gross),
            ("debt", debt_residual, &mut debt),
        ] {
            let value = &weight * &principal;
            *total += &value;
            if !weight.is_zero() {
                contributions.push(Contribution {
                    object_id: loan.id.clone(),
                    label: loan.id.clone(),
                    kind: kind.into(),
                    country: None,
                    category: "financement".into(),
                    weight: fraction_of(&weight),
                    amount_original: loan.principal.clone(),
                    currency: loan.currency.clone(),
                    valuation_id: None,
                    valuation_revision_id: None,
                    valuation_date: Some(r.date.clone()),
                    amount_eur: format_eur(&value),
                    exact_eur: fraction_of(&value),
                    source_ids: loan
                        .creditors
                        .iter()
                        .chain(&loan.debtors)
                        .map(|p| p.entity_id.clone())
                        .chain(std::iter::once(loan.id.clone()))
                        .chain(
                            conversion
                                .legs
                                .iter()
                                .flat_map(|leg| [leg.rate_id.clone(), leg.revision_id.clone()]),
                        )
                        .collect(),
                    conversion: Some(conversion.clone()),
                });
            }
        }
        if !eliminated.is_zero() {
            warn(
                &mut warnings,
                "internal_loan_elimination",
                &loan.id,
                &format!(
                    "Créance et dette appariées neutralisées de {} EUR, sans modifier le net.",
                    format_eur(&(&eliminated * &principal))
                ),
            );
        }
    }
    let underlying_complete = !opaque
        && warnings.iter().all(|w| {
            [
                "internal_loan_elimination",
                "contingent_guarantee",
                "undated_profile",
            ]
            .contains(&w.code.as_str())
        });
    if !liabilities_complete {
        warn(&mut warnings,"liabilities_not_inventoried","scope","Ce contrôle attribue uniquement les actifs enregistrés. Les financements, comptes et créances locatives ne sont pas encore intégrés à ce total : ce sous-total n’est pas un patrimoine net exhaustif.");
    }
    let net = &gross - &debt;
    check_size(&gross)?;
    check_size(&debt)?;
    check_size(&net)?;
    let status = if warnings.iter().any(|w| {
        ![
            "liabilities_not_inventoried",
            "internal_loan_elimination",
            "contingent_guarantee",
        ]
        .contains(&w.code.as_str())
    }) && contributions.is_empty()
    {
        "non_calculable"
    } else if warnings
        .iter()
        .any(|w| !["internal_loan_elimination", "contingent_guarantee"].contains(&w.code.as_str()))
    {
        "partial"
    } else {
        "complete"
    };
    Ok(ConsolidationReport {
        context: r.context.clone(),
        person_id: r.person_id.clone(),
        date: r.date.clone(),
        knowledge_sequence: s.knowledge_sequence,
        engine_version: "registry-fx-history-2".into(),
        status: status.into(),
        gross_eur: format_eur(&gross),
        debt_eur: format_eur(&debt),
        net_eur: format_eur(&net),
        exact_gross: fraction_of(&gross),
        exact_debt: fraction_of(&debt),
        exact_net: fraction_of(&net),
        global_equity_eur: format_eur(&global_equity),
        gross_is_underlying_complete: underlying_complete,
        contributions,
        warnings,
    })
}

fn behind_opaque_boundary(s: &RegistrySnapshot, entity: &str, date: &str) -> bool {
    let mut pending = vec![entity];
    let mut visited = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !visited.insert(id) {
            continue;
        }
        if policy(s, id, date).is_some_and(|p| p.mode == "global_equity") {
            return true;
        }
        for p in s
            .participations
            .iter()
            .filter(|p| p.company_id == id && applies(&p.valid_from, p.valid_to.as_deref(), date))
        {
            if p.economic_share.as_ref().is_none_or(|f| f.num != "0") {
                pending.push(&p.owner_entity_id);
            }
        }
    }
    false
}

fn party_weight(
    parties: &[DebtParty],
    weights: &BTreeMap<String, Weight>,
    s: &RegistrySnapshot,
) -> Result<(BigRational, bool)> {
    if parties.is_empty() {
        return Err(AppError::new(
            "validation",
            "Chaque face d’un prêt doit identifier au moins une partie.",
        ));
    }
    let mut sum = BigRational::zero();
    let mut weight = BigRational::zero();
    let mut unknown = false;
    let mut seen = BTreeSet::new();
    for party in parties {
        if !seen.insert(&party.entity_id) || !s.entities.iter().any(|e| e.id == party.entity_id) {
            return Err(AppError::new(
                "validation",
                "Une contrepartie du prêt est dupliquée ou inconnue.",
            ));
        }
        let parent = weights.get(&party.entity_id).cloned().unwrap_or_default();
        if let Some(share) = &party.economic_share {
            let share = parse_fraction(share)?;
            weight += &parent.known * &share;
            unknown |= parent.unknown && !share.is_zero();
            sum += share;
        } else {
            unknown = true;
        }
    }
    if sum > BigRational::one() {
        return Err(AppError::new(
            "validation",
            "Les allocations économiques d’une face du prêt dépassent 100 %.",
        ));
    }
    unknown |= sum < BigRational::one();
    Ok((weight, unknown))
}
