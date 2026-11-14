//! Financing contracts: one nominal source, two explicit economic faces.
use crate::*;
use serde::{Deserialize, Serialize};

macro_rules! debt_record {
    ($row:ident,$input:ident,{$($field:ident:$ty:ty),*$(,)?}) => {
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row { pub id:String,pub revision_id:String,$(pub $field:$ty,)* }
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input { pub id:Option<String>,pub expected_revision_id:Option<String>,pub reason:String,$(pub $field:$ty,)* }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FinancingParty {
    pub line_id: String,
    pub side: String,
    pub entity_id: String,
    pub legal_share: Option<Fraction>,
    pub economic_share: Option<Fraction>,
    pub liability_kind: String,
    pub notes: String,
}

debt_record!(DebtContract,SaveDebtContractInput,{
    name:String,currency:String,tracking_start:String,notes:String,archived:bool,
});
debt_record!(DebtTerms,SaveDebtTermsInput,{
    debt_id:String,valid_from:String,valid_to:Option<String>,principal_status:String,
    interest_rate:Option<String>,interest_convention:String,parties:Vec<FinancingParty>,notes:String,
});
debt_record!(DebtMovement,SaveDebtMovementInput,{
    debt_id:String,kind:String,effective_on:String,principal_change:String,
    interest_paid:String,fees_paid:String,unlinked_reason:String,notes:String,
});
debt_record!(DebtReceivableAdjustment,SaveDebtReceivableAdjustmentInput,{
    debt_id:String,creditor_entity_id:String,kind:String,effective_on:String,
    valuation_delta:String,source:String,notes:String,
});
debt_record!(DebtSchedule,SaveDebtScheduleInput,{
    debt_id:String,due_on:String,expected_principal:String,expected_interest:String,
    expected_fees:String,status:String,notes:String,
});
debt_record!(DebtPaymentAllocation,SaveDebtPaymentAllocationInput,{
    movement_id:String,schedule_id:String,effective_on:String,principal:String,interest:String,fees:String,
});
debt_record!(DebtAssetLink,SaveDebtAssetLinkInput,{
    debt_id:String,asset_id:String,relation_kind:String,attributed_principal_share:Option<Fraction>,
    valid_from:String,valid_to:Option<String>,notes:String,
});
debt_record!(DebtGuarantee,SaveDebtGuaranteeInput,{
    debt_id:String,guarantor_entity_id:String,limit_amount:Option<String>,valid_from:String,
    valid_to:Option<String>,notes:String,
});
debt_record!(DebtCoverage,SaveDebtCoverageInput,{
    debt_id:String,company_id:String,side:String,valid_from:String,valid_to:Option<String>,
    included:bool,source:String,notes:String,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebtSnapshot {
    pub contracts: Vec<DebtContract>,
    pub terms: Vec<DebtTerms>,
    pub movements: Vec<DebtMovement>,
    pub receivable_adjustments: Vec<DebtReceivableAdjustment>,
    pub schedules: Vec<DebtSchedule>,
    pub payment_allocations: Vec<DebtPaymentAllocation>,
    pub asset_links: Vec<DebtAssetLink>,
    pub guarantees: Vec<DebtGuarantee>,
    pub coverages: Vec<DebtCoverage>,
    pub knowledge_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DebtReportRequest {
    pub debt_id: String,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtBalance {
    pub debt_id: String,
    pub date: String,
    pub currency: String,
    pub principal_original: Option<String>,
    pub exact_principal: Option<Fraction>,
    pub interest_paid_original: String,
    pub exact_interest_paid: Fraction,
    pub fees_paid_original: String,
    pub exact_fees_paid: Fraction,
    pub opening_known: bool,
    pub source_ids: Vec<String>,
    pub warnings: Vec<DebtWarning>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtPartyPosition {
    pub party: FinancingParty,
    pub nominal_original: Option<String>,
    pub exact_nominal: Option<Fraction>,
    pub adjustment_original: String,
    pub exact_adjustment: Fraction,
    pub carrying_value_original: Option<String>,
    pub exact_carrying_value: Option<Fraction>,
    pub legal_maximum_original: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtScheduleStatus {
    pub schedule: DebtSchedule,
    pub paid_principal: String,
    pub paid_interest: String,
    pub paid_fees: String,
    pub remaining_principal: String,
    pub remaining_interest: String,
    pub remaining_fees: String,
    pub overdue_days: i64,
    pub settled_on: Option<String>,
    pub historical_late: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebtReport {
    pub debt_id: String,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub contract: DebtContract,
    pub terms: Option<DebtTerms>,
    pub balance: DebtBalance,
    pub principal_eur: Option<String>,
    pub exact_principal_eur: Option<Fraction>,
    pub conversion: Option<FxResult>,
    pub parties: Vec<DebtPartyPosition>,
    pub schedules: Vec<DebtScheduleStatus>,
    pub asset_links: Vec<DebtAssetLink>,
    pub guarantees: Vec<DebtGuarantee>,
    pub coverages: Vec<DebtCoverage>,
    pub source_ids: Vec<String>,
    pub warnings: Vec<DebtWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostedDebtCashLink {
    pub line_id: String,
    pub side: String,
    pub component: String,
    pub amount: String,
}
/// Creation or correction of the source and its exact cash links is one
/// transaction. The store rejects a source correction that would duplicate a
/// previously posted cash transaction rather than revising/reusing it.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostDebtMovementInput {
    pub expected_sequence: u64,
    pub movement: SaveDebtMovementInput,
    pub transaction: Option<SaveTreasuryTransactionInput>,
    pub existing_transaction_revision_id: Option<String>,
    pub links: Vec<PostedDebtCashLink>,
}

use crate::exact::{
    currency_minor_units, format_currency, format_eur, fraction_of, parse_decimal, parse_fraction,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        return Err(invalid("Un identifiant de financement est invalide."));
    }
    Ok(())
}
fn text(value: &str, maximum: usize, required: bool) -> Result<()> {
    if value.chars().count() > maximum
        || (required && value.trim().is_empty())
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid(
            "Une désignation de financement est vide, trop longue ou invalide.",
        ));
    }
    Ok(())
}
fn nonnegative(source: &str) -> Result<BigRational> {
    let value = parse_decimal(source)?;
    if value.is_negative() {
        Err(invalid("Le montant doit être positif ou nul."))
    } else {
        Ok(value)
    }
}
fn ratio(source: &Fraction) -> Result<BigRational> {
    let num = source
        .num
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction calculée invalide."))?;
    let den = source
        .den
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction calculée invalide."))?;
    if den <= BigInt::zero() {
        return Err(invalid("Fraction calculée invalide."));
    }
    Ok(BigRational::new(num, den))
}
fn warning(out: &mut Vec<DebtWarning>, code: &str, id: &str, message: &str) {
    if !out
        .iter()
        .any(|w| w.code == code && w.object_id == id && w.message == message)
    {
        out.push(DebtWarning {
            code: code.into(),
            object_id: id.into(),
            message: message.into(),
        });
    }
}
fn nonoverlapping(mut rows: Vec<(String, &str, Option<&str>)>) -> Result<()> {
    rows.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    if rows
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && pair[0].2.is_none_or(|end| pair[1].1 < end))
    {
        return Err(invalid(
            "Deux périodes de financement de même nature se chevauchent.",
        ));
    }
    Ok(())
}
fn active_terms<'a>(s: &'a DebtSnapshot, debt: &str, date: &str) -> Option<&'a DebtTerms> {
    s.terms
        .iter()
        .find(|t| t.debt_id == debt && applies(&t.valid_from, t.valid_to.as_deref(), date))
}
fn principal_source_at(s: &DebtSnapshot, debt: &str, date: &str) -> Result<Option<BigRational>> {
    if !s
        .movements
        .iter()
        .any(|m| m.debt_id == debt && m.kind == "opening" && m.effective_on.as_str() <= date)
    {
        return Ok(None);
    }
    Ok(Some(
        s.movements
            .iter()
            .filter(|m| m.debt_id == debt && m.effective_on.as_str() <= date)
            .try_fold(BigRational::zero(), |total, m| {
                Ok(total + parse_decimal(&m.principal_change)?)
            })?,
    ))
}
fn principal_at(s: &DebtSnapshot, debt: &str, date: &str) -> Result<Option<BigRational>> {
    if active_terms(s, debt, date).is_some_and(|t| t.principal_status == "unknown") {
        return Ok(None);
    }
    principal_source_at(s, debt, date)
}
fn adjustment_at(s: &DebtSnapshot, debt: &str, creditor: &str, date: &str) -> Result<BigRational> {
    s.receivable_adjustments
        .iter()
        .filter(|a| {
            a.debt_id == debt && a.creditor_entity_id == creditor && a.effective_on.as_str() <= date
        })
        .try_fold(BigRational::zero(), |total, a| {
            Ok(total + parse_decimal(&a.valuation_delta)?)
        })
}

/// All affected dates are checked before persistence. Missing openings and
/// allocations remain unknown rather than acquiring an assumed zero balance.
pub fn validate_debts(registry: &RegistrySnapshot, s: &DebtSnapshot) -> Result<()> {
    validate_registry(registry)?;
    if s.knowledge_sequence != registry.knowledge_sequence {
        return Err(invalid(
            "Les financements et le registre doivent partager la même borne de connaissance.",
        ));
    }
    let mut ids = HashSet::new();
    let mut revisions = HashSet::new();
    for (id, rev) in s
        .contracts
        .iter()
        .map(|v| (&v.id, &v.revision_id))
        .chain(s.terms.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.movements.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(
            s.receivable_adjustments
                .iter()
                .map(|v| (&v.id, &v.revision_id)),
        )
        .chain(s.schedules.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(
            s.payment_allocations
                .iter()
                .map(|v| (&v.id, &v.revision_id)),
        )
        .chain(s.asset_links.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.guarantees.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.coverages.iter().map(|v| (&v.id, &v.revision_id)))
    {
        identifier(id)?;
        identifier(rev)?;
        if !ids.insert(id) || !revisions.insert(rev) {
            return Err(invalid(
                "Une identité ou révision de financement apparaît plusieurs fois.",
            ));
        }
    }
    let contracts: HashMap<_, _> = s.contracts.iter().map(|c| (c.id.as_str(), c)).collect();
    let entities: HashMap<_, _> = registry
        .entities
        .iter()
        .map(|e| (e.id.as_str(), e))
        .collect();
    let assets: HashSet<_> = registry.assets.iter().map(|a| a.id.as_str()).collect();
    for c in &s.contracts {
        text(&c.name, 160, true)?;
        text(&c.notes, 10000, false)?;
        currency_minor_units(&c.currency)?;
        validate_date(&c.tracking_start)?;
    }
    let contract_date = |id: &str, date: &str| -> Result<()> {
        let c = contracts
            .get(id)
            .ok_or_else(|| invalid("Le contrat de financement est inconnu."))?;
        validate_date(date)?;
        if date < c.tracking_start.as_str() {
            return Err(invalid(
                "Un fait de financement précède le début du suivi du contrat.",
            ));
        }
        Ok(())
    };
    for t in &s.terms {
        contract_date(&t.debt_id, &t.valid_from)?;
        validate_interval(&t.valid_from, t.valid_to.as_deref())?;
        if !["complete", "unknown"].contains(&t.principal_status.as_str()) {
            return Err(invalid("La complétude du principal doit être explicite."));
        }
        if let Some(rate) = &t.interest_rate {
            nonnegative(rate)?;
        }
        text(&t.interest_convention, 1000, false)?;
        text(&t.notes, 10000, false)?;
        let mut lines = HashSet::new();
        let mut parties = HashSet::new();
        let mut sides: HashMap<&str, Vec<&FinancingParty>> = HashMap::new();
        for p in &t.parties {
            identifier(&p.line_id)?;
            if !lines.insert(&p.line_id)
                || !parties.insert((&p.side, &p.entity_id))
                || !entities.contains_key(p.entity_id.as_str())
                || !["creditor", "debtor"].contains(&p.side.as_str())
                || !["single", "several", "joint_and_several"].contains(&p.liability_kind.as_str())
            {
                return Err(invalid(
                    "Une contrepartie de financement est inconnue, ambiguë ou dupliquée.",
                ));
            }
            for share in [&p.legal_share, &p.economic_share].into_iter().flatten() {
                parse_fraction(share)?;
            }
            text(&p.notes, 2000, false)?;
            sides.entry(&p.side).or_default().push(p);
        }
        for side in ["creditor", "debtor"] {
            let rows = sides.get(side).ok_or_else(|| {
                invalid("Un contrat doit identifier au moins un créancier et un débiteur.")
            })?;
            let economic = rows
                .iter()
                .filter_map(|p| p.economic_share.as_ref())
                .try_fold(BigRational::zero(), |sum, v| Ok(sum + parse_fraction(v)?))?;
            if economic > BigRational::from_integer(BigInt::from(1)) {
                return Err(invalid(
                    "Les quotes-parts économiques d’un côté du contrat dépassent 100 %.",
                ));
            }
            if !rows.iter().any(|p| p.liability_kind == "joint_and_several") {
                let legal = rows
                    .iter()
                    .filter_map(|p| p.legal_share.as_ref())
                    .try_fold(BigRational::zero(), |sum, v| Ok(sum + parse_fraction(v)?))?;
                if legal > BigRational::from_integer(BigInt::from(1)) {
                    return Err(invalid("Les quotes-parts juridiques simples dépassent 100 %. La solidarité doit être qualifiée séparément."));
                }
            }
        }
        if t.parties.iter().filter(|p| p.side == "creditor").any(|c| {
            t.parties
                .iter()
                .any(|d| d.side == "debtor" && c.entity_id == d.entity_id)
        }) {
            return Err(invalid(
                "Une même entité ne peut pas être créancière et débitrice de son propre contrat.",
            ));
        }
    }
    nonoverlapping(
        s.terms
            .iter()
            .map(|t| {
                (
                    t.debt_id.clone(),
                    t.valid_from.as_str(),
                    t.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    let mut openings = HashSet::new();
    let mut dates: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for m in &s.movements {
        contract_date(&m.debt_id, &m.effective_on)?;
        text(&m.notes, 10000, false)?;
        text(&m.unlinked_reason, 2000, false)?;
        let p = parse_decimal(&m.principal_change)?;
        let interest = nonnegative(&m.interest_paid)?;
        let fees = nonnegative(&m.fees_paid)?;
        let is_cash = match m.kind.as_str() {
            "opening" => {
                if p.is_negative()
                    || !interest.is_zero()
                    || !fees.is_zero()
                    || m.effective_on != contracts[m.debt_id.as_str()].tracking_start
                    || !openings.insert(&m.debt_id)
                {
                    return Err(invalid("Le financement exige au plus une ouverture explicite au début du suivi, sans intérêts ni frais ajoutés au principal."));
                }
                false
            }
            "drawdown" if p > BigRational::zero() => true,
            "principal_repayment"
                if p <= BigRational::zero()
                    && (!p.is_zero() || !interest.is_zero() || !fees.is_zero()) =>
            {
                true
            }
            "principal_adjustment" if !p.is_zero() && interest.is_zero() && fees.is_zero() => {
                text(&m.notes, 10000, true)?;
                false
            }
            "write_off" if p < BigRational::zero() && interest.is_zero() && fees.is_zero() => {
                text(&m.notes, 10000, true)?;
                false
            }
            _ => {
                return Err(invalid(
                    "Le signe ou la nature du mouvement de principal est invalide.",
                ))
            }
        };
        if is_cash {
            text(&m.unlinked_reason, 2000, true)?;
        }
        dates.entry(&m.debt_id).or_default().insert(&m.effective_on);
    }
    for a in &s.receivable_adjustments {
        contract_date(&a.debt_id, &a.effective_on)?;
        if !entities.contains_key(a.creditor_entity_id.as_str()) {
            return Err(invalid("Le créancier de la dépréciation est inconnu."));
        }
        let delta = parse_decimal(&a.valuation_delta)?;
        if !((a.kind == "impairment" && delta < BigRational::zero())
            || (a.kind == "reversal" && delta > BigRational::zero()))
        {
            return Err(invalid(
                "Une dépréciation est négative ; une reprise est positive.",
            ));
        }
        text(&a.source, 1000, true)?;
        text(&a.notes, 10000, false)?;
        dates.entry(&a.debt_id).or_default().insert(&a.effective_on);
    }
    for t in &s.terms {
        let d = dates.entry(&t.debt_id).or_default();
        d.insert(&t.valid_from);
        if let Some(to) = &t.valid_to {
            d.insert(to);
        }
    }
    let budget = dates
        .iter()
        .try_fold(0_usize, |budget, (debt, boundaries)| {
            let rows = s.movements.iter().filter(|m| m.debt_id == **debt).count()
                + s.receivable_adjustments
                    .iter()
                    .filter(|a| a.debt_id == **debt)
                    .count()
                + s.terms.iter().filter(|t| t.debt_id == **debt).count();
            budget.checked_add(boundaries.len().saturating_mul(rows))
        })
        .unwrap_or(usize::MAX);
    if budget > 5_000_000 {
        return Err(AppError::new("capacity","La vérification temporelle des financements dépasse le budget de calcul. Aucun encours approximatif n’a été substitué."));
    }
    for (debt, boundaries) in &dates {
        for date in boundaries {
            if let Some(principal) = principal_source_at(s, debt, date)? {
                if principal.is_negative() {
                    return Err(invalid("Le capital restant dû devient négatif à une date affectée. Aucun remboursement n’a été plafonné silencieusement."));
                }
            }
            let creditors: HashSet<_> = s
                .receivable_adjustments
                .iter()
                .filter(|a| a.debt_id == *debt)
                .map(|a| a.creditor_entity_id.as_str())
                .collect();
            for creditor in creditors {
                let adjustment = adjustment_at(s, debt, creditor, date)?;
                if adjustment > BigRational::zero() {
                    return Err(invalid(
                        "Une reprise dépasse les dépréciations précédentes de cette créance.",
                    ));
                }
                if adjustment.is_zero() {
                    continue;
                }
                let principal=principal_at(s,debt,date)?.ok_or_else(||invalid("Une dépréciation exige un principal et des droits créanciers connus à toutes les dates affectées."))?;
                let share=active_terms(s,debt,date).and_then(|t|t.parties.iter().find(|p|p.side=="creditor" && p.entity_id==creditor)).and_then(|p|p.economic_share.as_ref()).ok_or_else(||invalid("Une dépréciation ne peut pas être transférée implicitement à un autre créancier ou à une allocation inconnue."))?;
                if -adjustment > principal * parse_fraction(share)? {
                    return Err(invalid("La dépréciation dépasse le reliquat nominal du créancier. Une reprise explicite est nécessaire."));
                }
            }
        }
    }
    let mut schedule_keys = HashSet::new();
    for schedule in &s.schedules {
        contract_date(&schedule.debt_id, &schedule.due_on)?;
        for source in [
            &schedule.expected_principal,
            &schedule.expected_interest,
            &schedule.expected_fees,
        ] {
            nonnegative(source)?;
        }
        text(&schedule.notes, 10000, false)?;
        if !["planned", "cancelled"].contains(&schedule.status.as_str())
            || !schedule_keys.insert((&schedule.debt_id, &schedule.due_on))
        {
            return Err(invalid(
                "L’échéance prévue est invalide ou concurrente à la même date.",
            ));
        }
    }
    let movements: HashMap<_, _> = s.movements.iter().map(|m| (m.id.as_str(), m)).collect();
    let schedules: HashMap<_, _> = s.schedules.iter().map(|m| (m.id.as_str(), m)).collect();
    let mut allocated_movements: HashMap<&str, [BigRational; 3]> = HashMap::new();
    let mut allocated_schedules: HashMap<&str, [BigRational; 3]> = HashMap::new();
    for allocation in &s.payment_allocations {
        validate_date(&allocation.effective_on)?;
        let movement = movements
            .get(allocation.movement_id.as_str())
            .ok_or_else(|| invalid("Le remboursement affecté est inconnu."))?;
        let schedule = schedules
            .get(allocation.schedule_id.as_str())
            .ok_or_else(|| invalid("L’échéance affectée est inconnue."))?;
        if movement.kind != "principal_repayment"
            || movement.debt_id != schedule.debt_id
            || schedule.status != "planned"
            || allocation.effective_on < movement.effective_on
        {
            return Err(invalid(
                "Le remboursement, l’échéance ou la date d’affectation sont incompatibles.",
            ));
        }
        let values = [
            nonnegative(&allocation.principal)?,
            nonnegative(&allocation.interest)?,
            nonnegative(&allocation.fees)?,
        ];
        if values.iter().all(Zero::is_zero) {
            return Err(invalid(
                "Une affectation de remboursement ne peut pas être entièrement nulle.",
            ));
        }
        for (index, value) in values.into_iter().enumerate() {
            allocated_movements
                .entry(&allocation.movement_id)
                .or_insert_with(|| std::array::from_fn(|_| BigRational::zero()))[index] += &value;
            allocated_schedules
                .entry(&allocation.schedule_id)
                .or_insert_with(|| std::array::from_fn(|_| BigRational::zero()))[index] += value;
        }
    }
    for (id, allocated) in allocated_movements {
        let movement = movements[id];
        let limits = [
            -parse_decimal(&movement.principal_change)?,
            parse_decimal(&movement.interest_paid)?,
            parse_decimal(&movement.fees_paid)?,
        ];
        if allocated
            .iter()
            .zip(limits)
            .any(|(used, limit)| used > &limit)
        {
            return Err(invalid(
                "Les affectations dépassent le principal, les intérêts ou frais réellement payés.",
            ));
        }
    }
    for (id, allocated) in allocated_schedules {
        let schedule = schedules[id];
        let limits = [
            parse_decimal(&schedule.expected_principal)?,
            parse_decimal(&schedule.expected_interest)?,
            parse_decimal(&schedule.expected_fees)?,
        ];
        if allocated
            .iter()
            .zip(limits)
            .any(|(used, limit)| used > &limit)
        {
            return Err(invalid(
                "Les affectations dépassent une composante de l’échéance prévue.",
            ));
        }
    }
    for link in &s.asset_links {
        contract_date(&link.debt_id, &link.valid_from)?;
        validate_interval(&link.valid_from, link.valid_to.as_deref())?;
        if !assets.contains(link.asset_id.as_str())
            || !["finances", "collateral"].contains(&link.relation_kind.as_str())
        {
            return Err(invalid(
                "Le bien financé ou la nature de son lien est inconnu.",
            ));
        }
        if let Some(share) = &link.attributed_principal_share {
            parse_fraction(share)?;
            if link.relation_kind == "collateral" {
                return Err(invalid(
                    "Une sûreté n’attribue pas automatiquement une fraction du principal au bien.",
                ));
            }
        }
        text(&link.notes, 10000, false)?;
    }
    nonoverlapping(
        s.asset_links
            .iter()
            .map(|l| {
                (
                    format!("{}|{}|{}", l.debt_id, l.asset_id, l.relation_kind),
                    l.valid_from.as_str(),
                    l.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    let link_boundaries: BTreeSet<_> = s
        .asset_links
        .iter()
        .flat_map(|l| std::iter::once(l.valid_from.as_str()).chain(l.valid_to.as_deref()))
        .collect();
    if link_boundaries.len().saturating_mul(s.asset_links.len()) > 5_000_000 {
        return Err(AppError::new(
            "capacity",
            "La vérification des affectations de dette dépasse le budget de calcul.",
        ));
    }
    for date in link_boundaries {
        let mut shares: HashMap<&str, BigRational> = HashMap::new();
        for l in s.asset_links.iter().filter(|l| {
            l.relation_kind == "finances" && applies(&l.valid_from, l.valid_to.as_deref(), date)
        }) {
            if let Some(share) = &l.attributed_principal_share {
                *shares.entry(&l.debt_id).or_insert_with(BigRational::zero) +=
                    parse_fraction(share)?;
            }
        }
        if shares
            .values()
            .any(|n| n > &BigRational::from_integer(BigInt::from(1)))
        {
            return Err(invalid(
                "Les affectations analytiques de principal aux biens dépassent 100 %.",
            ));
        }
    }
    for g in &s.guarantees {
        contract_date(&g.debt_id, &g.valid_from)?;
        validate_interval(&g.valid_from, g.valid_to.as_deref())?;
        if !entities.contains_key(g.guarantor_entity_id.as_str()) {
            return Err(invalid("Le garant est inconnu."));
        }
        if let Some(limit) = &g.limit_amount {
            nonnegative(limit)?;
        }
        text(&g.notes, 10000, false)?;
    }
    nonoverlapping(
        s.guarantees
            .iter()
            .map(|g| {
                (
                    format!("{}|{}", g.debt_id, g.guarantor_entity_id),
                    g.valid_from.as_str(),
                    g.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    for coverage in &s.coverages {
        contract_date(&coverage.debt_id, &coverage.valid_from)?;
        validate_interval(&coverage.valid_from, coverage.valid_to.as_deref())?;
        if !entities
            .get(coverage.company_id.as_str())
            .is_some_and(|e| e.kind == "company")
            || !["creditor", "debtor"].contains(&coverage.side.as_str())
        {
            return Err(invalid(
                "La couverture exige une société et une face du contrat identifiées.",
            ));
        }
        text(&coverage.source, 1000, true)?;
        text(&coverage.notes, 10000, false)?;
    }
    nonoverlapping(
        s.coverages
            .iter()
            .map(|c| {
                (
                    format!("{}|{}|{}", c.debt_id, c.company_id, c.side),
                    c.valid_from.as_str(),
                    c.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    Ok(())
}

fn balance_unchecked(s: &DebtSnapshot, contract: &DebtContract, date: &str) -> Result<DebtBalance> {
    let mut sources = BTreeSet::from([contract.id.clone(), contract.revision_id.clone()]);
    let mut interest = BigRational::zero();
    let mut fees = BigRational::zero();
    for m in s
        .movements
        .iter()
        .filter(|m| m.debt_id == contract.id && m.effective_on.as_str() <= date)
    {
        sources.extend([m.id.clone(), m.revision_id.clone()]);
        interest += parse_decimal(&m.interest_paid)?;
        fees += parse_decimal(&m.fees_paid)?;
    }
    let opening_known = s.movements.iter().any(|m| {
        m.debt_id == contract.id && m.kind == "opening" && m.effective_on.as_str() <= date
    });
    let principal = principal_at(s, &contract.id, date)?;
    let mut warnings = Vec::new();
    if !opening_known {
        warning(&mut warnings,"missing_debt_opening",&contract.id,"Le principal est inconnu sans ouverture explicite, y compris une ouverture nulle. Les mouvements antérieurs ne sont pas réinventés.");
    }
    if let Some(t) = active_terms(s, &contract.id, date) {
        sources.extend([t.id.clone(), t.revision_id.clone()]);
        if t.principal_status == "unknown" {
            warning(
                &mut warnings,
                "unknown_debt_principal",
                &contract.id,
                "Les conditions déclarent le principal incomplet à cette date.",
            );
        }
    } else {
        warning(
            &mut warnings,
            "missing_debt_terms",
            &contract.id,
            "Les parties et conditions de ce financement ne sont pas définies à cette date.",
        );
    }
    Ok(DebtBalance {
        debt_id: contract.id.clone(),
        date: date.into(),
        currency: contract.currency.clone(),
        principal_original: principal
            .as_ref()
            .map(|p| format_currency(p, &contract.currency))
            .transpose()?,
        exact_principal: principal.as_ref().map(fraction_of),
        interest_paid_original: format_currency(&interest, &contract.currency)?,
        exact_interest_paid: fraction_of(&interest),
        fees_paid_original: format_currency(&fees, &contract.currency)?,
        exact_fees_paid: fraction_of(&fees),
        opening_known,
        source_ids: sources.into_iter().collect(),
        warnings,
    })
}

pub fn debt_balance(
    registry: &RegistrySnapshot,
    s: &DebtSnapshot,
    debt_id: &str,
    date: &str,
) -> Result<DebtBalance> {
    validate_debts(registry, s)?;
    validate_date(date)?;
    let contract = s
        .contracts
        .iter()
        .find(|c| c.id == debt_id)
        .ok_or_else(|| invalid("Le contrat de financement est inconnu."))?;
    balance_unchecked(s, contract, date)
}

fn civil_date(source: &str) -> Result<time::Date> {
    validate_date(source)?;
    let year = source[..4]
        .parse()
        .map_err(|_| invalid("Année invalide."))?;
    let month = source[5..7]
        .parse::<u8>()
        .map_err(|_| invalid("Mois invalide."))?;
    let day = source[8..].parse().map_err(|_| invalid("Jour invalide."))?;
    time::Date::from_calendar_date(
        year,
        time::Month::try_from(month).map_err(|_| invalid("Mois invalide."))?,
        day,
    )
    .map_err(|_| invalid("Date invalide."))
}

pub fn debt_report(
    registry: &RegistrySnapshot,
    s: &DebtSnapshot,
    q: &DebtReportRequest,
) -> Result<DebtReport> {
    validate_debts(registry, s)?;
    validate_date(&q.date)?;
    if q.knowledge_sequence
        .is_some_and(|k| k != s.knowledge_sequence)
    {
        return Err(invalid(
            "La borne de connaissance ne correspond pas aux sources de financement.",
        ));
    }
    let contract = s
        .contracts
        .iter()
        .find(|c| c.id == q.debt_id)
        .ok_or_else(|| invalid("Le contrat de financement est inconnu."))?;
    let balance = balance_unchecked(s, contract, &q.date)?;
    let terms = active_terms(s, &q.debt_id, &q.date).cloned();
    let principal = balance.exact_principal.as_ref().map(ratio).transpose()?;
    let mut warnings = balance.warnings.clone();
    let mut sources: BTreeSet<_> = balance.source_ids.iter().cloned().collect();
    let mut parties = Vec::new();
    let mut incomplete = terms.is_none();
    if let Some(t) = &terms {
        for side in ["creditor", "debtor"] {
            let rows: Vec<_> = t.parties.iter().filter(|p| p.side == side).collect();
            let sum = rows
                .iter()
                .filter_map(|p| p.economic_share.as_ref())
                .try_fold(BigRational::zero(), |sum, s| Ok(sum + parse_fraction(s)?))?;
            if rows.iter().any(|p| p.economic_share.is_none())
                || sum != BigRational::from_integer(BigInt::from(1))
            {
                incomplete = true;
                warning(&mut warnings,"unknown_debt_allocation",&contract.id,"Les quotes-parts économiques d’un côté du contrat sont incomplètes. Aucune allocation suivant le bien financé n’a été déduite.");
            }
        }
        for p in &t.parties {
            sources.extend([p.entity_id.clone(), p.line_id.clone()]);
            let nominal = principal
                .as_ref()
                .zip(p.economic_share.as_ref())
                .map(|(n, f)| Ok(n * parse_fraction(f)?))
                .transpose()?;
            let adjustment = if p.side == "creditor" {
                adjustment_at(s, &contract.id, &p.entity_id, &q.date)?
            } else {
                BigRational::zero()
            };
            let carrying = nominal.as_ref().map(|n| n + &adjustment);
            let legal = if p.liability_kind == "joint_and_several" {
                principal.clone()
            } else {
                principal
                    .as_ref()
                    .zip(p.legal_share.as_ref())
                    .map(|(n, f)| Ok(n * parse_fraction(f)?))
                    .transpose()?
            };
            if p.liability_kind == "joint_and_several" {
                warning(&mut warnings,"joint_liability_exposure",&p.entity_id,"L’exposition juridique solidaire est présentée séparément ; elle ne remplace pas la quote-part économique patrimoniale.");
            }
            parties.push(DebtPartyPosition {
                party: p.clone(),
                nominal_original: nominal
                    .as_ref()
                    .map(|n| format_currency(n, &contract.currency))
                    .transpose()?,
                exact_nominal: nominal.as_ref().map(fraction_of),
                adjustment_original: format_currency(&adjustment, &contract.currency)?,
                exact_adjustment: fraction_of(&adjustment),
                carrying_value_original: carrying
                    .as_ref()
                    .map(|n| format_currency(n, &contract.currency))
                    .transpose()?,
                exact_carrying_value: carrying.as_ref().map(fraction_of),
                legal_maximum_original: legal
                    .as_ref()
                    .map(|n| format_currency(n, &contract.currency))
                    .transpose()?,
            });
        }
    }
    let mut conversion = None;
    let mut principal_eur = None;
    let mut exact_principal_eur = None;
    if let Some(principal) = &principal {
        let fx = crate::fx::convert_currency_unchecked(
            registry,
            &FxQuery {
                amount: "1".into(),
                from_currency: contract.currency.clone(),
                to_currency: "EUR".into(),
                date: q.date.clone(),
                knowledge_sequence: Some(s.knowledge_sequence),
            },
        )?;
        for w in &fx.warnings {
            warning(&mut warnings, &w.code, &contract.id, &w.message);
        }
        for leg in &fx.legs {
            sources.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
        }
        if let Some(rate) = &fx.exact_rate {
            let eur = principal * ratio(rate)?;
            principal_eur = Some(format_eur(&eur));
            exact_principal_eur = Some(fraction_of(&eur));
        } else {
            incomplete = true;
        }
        conversion = Some(fx);
    }
    let mut schedule_statuses = Vec::new();
    for schedule in s.schedules.iter().filter(|p| p.debt_id == contract.id) {
        sources.extend([schedule.id.clone(), schedule.revision_id.clone()]);
        let mut paid: [BigRational; 3] = std::array::from_fn(|_| BigRational::zero());
        let mut events: Vec<_> = s
            .payment_allocations
            .iter()
            .filter(|a| a.schedule_id == schedule.id && a.effective_on <= q.date)
            .collect();
        events.sort_by(|a, b| (&a.effective_on, &a.id).cmp(&(&b.effective_on, &b.id)));
        let expected = [
            parse_decimal(&schedule.expected_principal)?,
            parse_decimal(&schedule.expected_interest)?,
            parse_decimal(&schedule.expected_fees)?,
        ];
        let mut late = false;
        let mut settled = None;
        for a in events {
            sources.extend([a.id.clone(), a.revision_id.clone(), a.movement_id.clone()]);
            for (index, source) in [&a.principal, &a.interest, &a.fees].into_iter().enumerate() {
                paid[index] += parse_decimal(source)?;
            }
            late |= a.effective_on > schedule.due_on;
            if paid.iter().zip(&expected).all(|(p, e)| p == e) {
                settled = Some(a.effective_on.clone());
            }
        }
        let remaining: [BigRational; 3] = std::array::from_fn(|i| &expected[i] - &paid[i]);
        let overdue = schedule.status == "planned"
            && schedule.due_on < q.date
            && remaining.iter().any(|v| !v.is_zero());
        late |= overdue;
        schedule_statuses.push(DebtScheduleStatus {
            schedule: schedule.clone(),
            paid_principal: format_currency(&paid[0], &contract.currency)?,
            paid_interest: format_currency(&paid[1], &contract.currency)?,
            paid_fees: format_currency(&paid[2], &contract.currency)?,
            remaining_principal: format_currency(&remaining[0], &contract.currency)?,
            remaining_interest: format_currency(&remaining[1], &contract.currency)?,
            remaining_fees: format_currency(&remaining[2], &contract.currency)?,
            overdue_days: if overdue {
                (civil_date(&q.date)? - civil_date(&schedule.due_on)?).whole_days()
            } else {
                0
            },
            settled_on: settled,
            historical_late: late,
        });
    }
    schedule_statuses.sort_by(|a, b| {
        (&a.schedule.due_on, &a.schedule.id).cmp(&(&b.schedule.due_on, &b.schedule.id))
    });
    let guarantees: Vec<_> = s
        .guarantees
        .iter()
        .filter(|g| {
            g.debt_id == contract.id && applies(&g.valid_from, g.valid_to.as_deref(), &q.date)
        })
        .cloned()
        .collect();
    for g in &guarantees {
        sources.extend([
            g.id.clone(),
            g.revision_id.clone(),
            g.guarantor_entity_id.clone(),
        ]);
        warning(&mut warnings,"contingent_guarantee",&g.id,"Cette garantie est un engagement conditionnel, hors dettes certaines. Aucun appel n’a été supposé.");
    }
    let asset_links: Vec<_> = s
        .asset_links
        .iter()
        .filter(|l| {
            l.debt_id == contract.id && applies(&l.valid_from, l.valid_to.as_deref(), &q.date)
        })
        .cloned()
        .collect();
    for l in &asset_links {
        sources.extend([l.id.clone(), l.revision_id.clone(), l.asset_id.clone()]);
    }
    let coverages: Vec<_> = s
        .coverages
        .iter()
        .filter(|c| {
            c.debt_id == contract.id && applies(&c.valid_from, c.valid_to.as_deref(), &q.date)
        })
        .cloned()
        .collect();
    for c in &coverages {
        sources.extend([c.id.clone(), c.revision_id.clone(), c.company_id.clone()]);
    }
    for a in s
        .receivable_adjustments
        .iter()
        .filter(|a| a.debt_id == contract.id && a.effective_on <= q.date)
    {
        sources.extend([a.id.clone(), a.revision_id.clone()]);
    }
    warning(&mut warnings,"debt_cash_link_scope",&contract.id,"Le principal, les intérêts et frais sont distincts. Le rapprochement de trésorerie et les éliminations du périmètre se contrôlent dans le rapport financier, sans seconde écriture implicite.");
    Ok(DebtReport {
        debt_id: contract.id.clone(),
        date: q.date.clone(),
        knowledge_sequence: s.knowledge_sequence,
        status: if principal.is_none() {
            "non_calculable"
        } else if incomplete {
            "partial"
        } else {
            "complete"
        }
        .into(),
        contract: contract.clone(),
        terms,
        balance,
        principal_eur,
        exact_principal_eur,
        conversion,
        parties,
        schedules: schedule_statuses,
        asset_links,
        guarantees,
        coverages,
        source_ids: sources.into_iter().collect(),
        warnings,
    })
}
