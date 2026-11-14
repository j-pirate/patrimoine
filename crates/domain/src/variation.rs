//! Reproducible bridge for the recorded asset subtotal, without a return claim.
//!
//! Interactions follow the fixed order scope/method, rights, valuation, then FX.
//! Every step is the difference of two exact recalculations. A missing reference
//! leaves the step unknown; it is never replaced with a zero contribution.
use crate::consolidation::consolidate_counterfactual;
use crate::exact::{format_eur, fraction_of};
use crate::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VariationRequest {
    pub context: String,
    pub person_id: Option<String>,
    pub from_date: String,
    pub to_date: String,
    pub knowledge_sequence: Option<u64>,
}

/// Dates held fixed in a counterfactual, all interpreted at the same K.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationDates {
    pub membership_date: String,
    pub method_date: String,
    pub rights_date: String,
    pub valuation_date: String,
    pub fx_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationStep {
    pub code: String,
    pub label: String,
    pub amount_eur: Option<String>,
    pub exact_amount: Option<Fraction>,
    /// Known recorded subtotals; these are not complete values when warned.
    pub before_eur: String,
    pub after_eur: String,
    pub dates: VariationDates,
    pub source_ids: Vec<String>,
    pub warnings: Vec<ConsolidationWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariationReport {
    pub from_date: String,
    pub to_date: String,
    pub knowledge_sequence: u64,
    pub opening: ConsolidationReport,
    pub closing: ConsolidationReport,
    pub steps: Vec<VariationStep>,
    /// Difference of the known recorded asset subtotals, even when partial.
    pub delta_eur: String,
    pub exact_delta: Fraction,
    pub explained_eur: String,
    pub exact_explained: Fraction,
    /// Unattributed difference of recorded subtotals, not an estimated gain.
    pub residual_eur: Option<String>,
    pub exact_residual: Option<Fraction>,
    pub status: String,
    pub warnings: Vec<ConsolidationWarning>,
}

fn exact(value: &Fraction) -> Result<BigRational> {
    let error = || AppError::new("integrity", "Le calcul exact du sous-total est invalide.");
    let numerator = BigInt::from_str(&value.num).map_err(|_| error())?;
    let denominator = BigInt::from_str(&value.den).map_err(|_| error())?;
    if denominator <= BigInt::zero() {
        return Err(error());
    }
    Ok(BigRational::new(numerator, denominator))
}

fn missing_reference(warning: &ConsolidationWarning) -> bool {
    [
        "missing_valuation",
        "missing_exchange_rate",
        "unknown_economic_rights",
        "missing_company_policy",
        "incomplete_company_coverage",
    ]
    .contains(&warning.code.as_str())
}

fn add_warnings(target: &mut Vec<ConsolidationWarning>, source: &[ConsolidationWarning]) {
    for warning in source {
        if !target.iter().any(|w| {
            w.code == warning.code
                && w.object_id == warning.object_id
                && w.message == warning.message
        }) {
            target.push(warning.clone());
        }
    }
}

/// Explain the change in the asset subtotal recorded in one knowledge snapshot.
/// This does not infer cash flows, acquisition costs, personal liabilities, or
/// an investment return. Rights changes are shown explicitly as their own step.
pub fn explain_variation(s: &RegistrySnapshot, q: &VariationRequest) -> Result<VariationReport> {
    validate_date(&q.from_date)?;
    validate_date(&q.to_date)?;
    if q.to_date < q.from_date {
        return Err(AppError::new(
            "validation",
            "La date de fin du pont doit être postérieure ou égale au début.",
        ));
    }
    let d0 = q.from_date.as_str();
    let d1 = q.to_date.as_str();
    let dates = [
        [d0, d0, d0, d0, d0],
        [d1, d1, d0, d0, d0],
        [d1, d1, d1, d0, d0],
        [d1, d1, d1, d1, d0],
        [d1, d1, d1, d1, d1],
    ];
    let mut request = ConsolidationRequest {
        context: q.context.clone(),
        person_id: q.person_id.clone(),
        date: q.to_date.clone(),
        knowledge_sequence: q.knowledge_sequence,
    };
    let mut reports = Vec::with_capacity(dates.len());
    for dates in dates {
        // Metadata and the report observation date follow the rights date;
        // notably the opening must not display the closing civil profiles.
        request.date = dates[2].to_owned();
        let mut report = consolidate_counterfactual(
            s, &request, dates[0], dates[1], dates[2], dates[3], dates[4],
        )?;
        // Public bookends retain their actual observation date. The mixed
        // stages publish the independently fixed dates in VariationStep.
        report.date = dates[2].to_owned();
        reports.push(report);
    }
    let descriptions = [
        ("scope_method", "Périmètre familial et méthodes"),
        ("rights", "Droits économiques et actifs suivis"),
        ("valuation", "Prix et estimations"),
        ("fx", "Change"),
    ];
    let mut steps = Vec::with_capacity(descriptions.len());
    let mut explained = BigRational::zero();
    let mut warnings = Vec::new();
    let mut has_unknown = false;
    for (index, (code, label)) in descriptions.into_iter().enumerate() {
        let before = &reports[index];
        let after = &reports[index + 1];
        let mut step_warnings = Vec::new();
        add_warnings(&mut step_warnings, &before.warnings);
        add_warnings(&mut step_warnings, &after.warnings);
        let known = !step_warnings.iter().any(missing_reference);
        let amount = exact(&after.exact_gross)? - exact(&before.exact_gross)?;
        if known {
            explained += &amount;
        } else {
            has_unknown = true;
            step_warnings.push(ConsolidationWarning {
                code: "variation_reference_missing".into(),
                object_id: code.into(),
                message: "Une référence de ce recalcul manque ou son périmètre est incomplet. L’effet reste inconnu ; la différence des sous-totaux enregistrés ne constitue pas une performance.".into(),
            });
        }
        add_warnings(&mut warnings, &step_warnings);
        let sources: BTreeSet<_> = before
            .contributions
            .iter()
            .chain(&after.contributions)
            .flat_map(|c| c.source_ids.iter().cloned())
            .collect();
        let d = dates[index + 1];
        steps.push(VariationStep {
            code: code.into(),
            label: label.into(),
            amount_eur: known.then(|| format_eur(&amount)),
            exact_amount: known.then(|| fraction_of(&amount)),
            before_eur: before.gross_eur.clone(),
            after_eur: after.gross_eur.clone(),
            dates: VariationDates {
                membership_date: d[0].into(),
                method_date: d[1].into(),
                rights_date: d[2].into(),
                valuation_date: d[3].into(),
                fx_date: d[4].into(),
            },
            source_ids: sources.into_iter().collect(),
            warnings: step_warnings,
        });
    }
    let closing = reports.pop().expect("five counterfactuals");
    let opening = reports.remove(0);
    let delta = exact(&closing.exact_gross)? - exact(&opening.exact_gross)?;
    let residual = &delta - &explained;
    if has_unknown {
        warnings.push(ConsolidationWarning {
            code: "variation_partial_subtotal".into(),
            object_id: "scope".into(),
            message: "Le pont porte sur les sous-totaux d’actifs enregistrés. Des références manquent : le résidu est une différence non attribuable de ces sous-totaux, sans estimation du patrimoine manquant ni d’un rendement.".into(),
        });
    }
    Ok(VariationReport {
        from_date: q.from_date.clone(),
        to_date: q.to_date.clone(),
        knowledge_sequence: s.knowledge_sequence,
        opening,
        closing,
        steps,
        delta_eur: format_eur(&delta),
        exact_delta: fraction_of(&delta),
        explained_eur: format_eur(&explained),
        exact_explained: fraction_of(&explained),
        residual_eur: Some(format_eur(&residual)),
        exact_residual: Some(fraction_of(&residual)),
        status: if has_unknown { "partial" } else { "complete" }.into(),
        warnings,
    })
}
