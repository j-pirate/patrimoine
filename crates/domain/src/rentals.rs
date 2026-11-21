//! Locations, échéances et affectations : sources exactes et séparées des banques.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

macro_rules! rental_record {
    ($row:ident,$input:ident,{$($field:ident:$ty:ty),*$(,)?}) => {
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row { pub id:String,pub revision_id:String,$(pub $field:$ty,)* }
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input { pub id:Option<String>,pub expected_revision_id:Option<String>,pub reason:String,$(pub $field:$ty,)* }
    }
}

rental_record!(Lease,SaveLeaseInput,{
    unit_id:String,tenant_entity_id:String,valid_from:String,valid_to:Option<String>,notes:String,
});
rental_record!(LeaseTerms,SaveLeaseTermsInput,{
    lease_id:String,rent:String,charges:String,currency:String,frequency:String,due_day:u8,
    deposit_required:String,valid_from:String,valid_to:Option<String>,notes:String,
});
rental_record!(RentDue,SaveRentDueInput,{
    lease_id:String,terms_revision_id:String,period_start:String,period_end:String,due_on:String,
    rent:String,charges:String,currency:String,generation_key:String,notes:String,
});
rental_record!(RentPayment,SaveRentPaymentInput,{
    lease_id:String,kind:String,amount:String,currency:String,effective_on:String,
    original_payment_id:Option<String>,recipient_entity_id:String,reference:String,unlinked_reason:String,
});
rental_record!(RentAllocation,SaveRentAllocationInput,{
    payment_id:String,due_id:String,component:String,amount:String,currency:String,effective_on:String,
});
rental_record!(RentalExpense,SaveRentalExpenseInput,{
    asset_id:String,kind:String,amount:String,currency:String,effective_on:String,paid_on:Option<String>,
    recoverable:bool,notes:String,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RentalSnapshot {
    pub leases: Vec<Lease>,
    pub lease_terms: Vec<LeaseTerms>,
    pub lease_term_revisions: Vec<LeaseTerms>,
    pub dues: Vec<RentDue>,
    pub payments: Vec<RentPayment>,
    pub allocations: Vec<RentAllocation>,
    pub expenses: Vec<RentalExpense>,
    pub knowledge_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateRentDuesInput {
    pub lease_id: String,
    pub from_date: String,
    pub to_date: String,
    pub expected_sequence: u64,
    pub reason: String,
}

/// A source proposal has no UUID; the store assigns one only for an absent key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneratedRentDue {
    pub lease_id: String,
    pub terms_revision_id: String,
    pub period_start: String,
    pub period_end: String,
    pub due_on: String,
    pub rent: String,
    pub charges: String,
    pub currency: String,
    pub generation_key: String,
    pub notes: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentGenerationResult {
    pub created: Vec<RentDue>,
    pub existing_count: u64,
    pub knowledge_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RentalReportRequest {
    #[serde(default)]
    pub country: Option<String>,
    pub context: String,
    pub person_id: Option<String>,
    pub asset_id: Option<String>,
    pub from_date: String,
    pub to_date: String,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentDueStatus {
    pub due_id: String,
    pub lease_id: String,
    pub asset_id: String,
    pub due_on: String,
    pub period_start: String,
    pub period_end: String,
    pub currency: String,
    pub rent: String,
    pub charges: String,
    pub allocated_rent: String,
    pub allocated_charges: String,
    pub outstanding_rent: String,
    pub outstanding_charges: String,
    pub overdue_days: i64,
    pub settled_on: Option<String>,
    pub historical_late: bool,
    pub source_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalOccupation {
    pub occupied_unit_days: u64,
    pub available_unit_days: u64,
    pub percent: Option<String>,
    pub basis: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RentalReport {
    pub source_ids: Vec<String>,
    pub conversions: Vec<FxResult>,
    pub from_date: String,
    pub to_date: String,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub contractual_annual_rent_eur: Option<String>,
    pub rent_due_eur: Option<String>,
    pub charges_due_eur: Option<String>,
    pub rent_received_eur: Option<String>,
    pub charges_received_eur: Option<String>,
    pub outstanding_rent_eur: Option<String>,
    pub outstanding_charges_eur: Option<String>,
    pub unallocated_payments_eur: Option<String>,
    pub economic_rent_eur: Option<String>,
    pub operating_expenses_eur: Option<String>,
    pub noi_eur: Option<String>,
    pub gross_yield_percent: Option<String>,
    pub net_yield_percent: Option<String>,
    pub cash_flow_after_financing_eur: Option<String>,
    pub exact_totals: BTreeMap<String, Fraction>,
    pub occupation: RentalOccupation,
    pub dues: Vec<RentDueStatus>,
    pub warnings: Vec<RentalWarning>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DepositEffect {
    pub cash_increase: Fraction,
    pub restitution_liability: Fraction,
    pub rental_income: Fraction,
}

/// An explicit settlement without a new cash receipt (for example applying a
/// held deposit). The original cash payment records remain unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NonCashRentSettlement {
    pub due_id: String,
    pub component: String,
    pub amount: String,
    pub currency: String,
    pub effective_on: String,
    pub source_ids: Vec<String>,
}

use crate::exact::{
    currency_minor_units, format_currency, format_eur, fraction_of, normalize_decimal_input,
    parse_decimal,
};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use std::collections::{BTreeSet, HashMap, HashSet};

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn identifier(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("Identifiant locatif invalide."));
    }
    Ok(())
}
fn text(s: &str, limit: usize, required: bool) -> Result<()> {
    if s.chars().count() > limit
        || (required && s.trim().is_empty())
        || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid("Un texte locatif est vide, trop long ou invalide."));
    }
    Ok(())
}
fn nonnegative(s: &str) -> Result<BigRational> {
    let n = parse_decimal(s)?;
    if n.is_negative() {
        Err(invalid("Le montant locatif ne peut pas être négatif."))
    } else {
        Ok(n)
    }
}
fn date(s: &str) -> Result<time::Date> {
    validate_date(s)?;
    time::Date::from_calendar_date(
        s[..4].parse().map_err(|_| invalid("Année invalide."))?,
        time::Month::try_from(
            s[5..7]
                .parse::<u8>()
                .map_err(|_| invalid("Mois invalide."))?,
        )
        .map_err(|_| invalid("Mois invalide."))?,
        s[8..].parse().map_err(|_| invalid("Jour invalide."))?,
    )
    .map_err(|_| invalid("Date invalide."))
}
fn ds(d: time::Date) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
}
fn contained(start: &str, end: Option<&str>, outer_start: &str, outer_end: Option<&str>) -> bool {
    start >= outer_start && outer_end.is_none_or(|outer| end.is_some_and(|inner| inner <= outer))
}
fn disjoint(mut rows: Vec<(&str, &str, Option<&str>)>) -> Result<()> {
    rows.sort();
    for pair in rows.windows(2) {
        if pair[0].0 == pair[1].0 && pair[0].2.is_none_or(|end| pair[1].1 < end) {
            return Err(invalid(
                "Deux périodes locatives se chevauchent pour le même objet.",
            ));
        }
    }
    Ok(())
}
fn canonical_money(n: &BigRational, currency: &str) -> Result<String> {
    normalize_decimal_input(&format_currency(n, currency)?)
}
fn allocated(s: &RentalSnapshot, due: &str, component: &str, as_of: &str) -> Result<BigRational> {
    s.allocations
        .iter()
        .filter(|a| a.due_id == due && a.component == component && a.effective_on.as_str() <= as_of)
        .try_fold(BigRational::zero(), |sum, a| {
            Ok(sum + parse_decimal(&a.amount)?)
        })
}

fn allocated_for_report(
    s: &RentalSnapshot,
    settlements: &[NonCashRentSettlement],
    due: &str,
    component: &str,
    as_of: &str,
) -> Result<BigRational> {
    settlements
        .iter()
        .filter(|v| v.due_id == due && v.component == component && v.effective_on.as_str() <= as_of)
        .try_fold(allocated(s, due, component, as_of)?, |sum, v| {
            Ok(sum + parse_decimal(&v.amount)?)
        })
}

fn validate_non_cash_settlements(
    s: &RentalSnapshot,
    settlements: &[NonCashRentSettlement],
) -> Result<()> {
    if settlements
        .len()
        .saturating_mul(s.dues.len().saturating_add(s.allocations.len()))
        > 5_000_000
    {
        return Err(AppError::new(
            "capacity",
            "Le contrôle des règlements non monétaires dépasse le budget de calcul.",
        ));
    }
    let mut timelines: BTreeMap<(&str, &str), BTreeMap<&str, BigRational>> = BTreeMap::new();
    for a in &s.allocations {
        *timelines
            .entry((&a.due_id, &a.component))
            .or_default()
            .entry(&a.effective_on)
            .or_insert_with(BigRational::zero) += parse_decimal(&a.amount)?;
    }
    for settlement in settlements {
        let due = s
            .dues
            .iter()
            .find(|d| d.id == settlement.due_id)
            .ok_or_else(|| {
                invalid("Le règlement sans nouvel encaissement concerne une échéance inconnue.")
            })?;
        validate_date(&settlement.effective_on)?;
        if !["rent", "charges"].contains(&settlement.component.as_str())
            || settlement.currency != due.currency
            || settlement.effective_on < due.due_on
            || parse_decimal(&settlement.amount)? <= BigRational::zero()
        {
            return Err(invalid("Le règlement non monétaire doit conserver la devise, une composante positive et l’exigibilité de son échéance."));
        }
        for id in &settlement.source_ids {
            identifier(id)?;
        }
        *timelines
            .entry((&settlement.due_id, &settlement.component))
            .or_default()
            .entry(&settlement.effective_on)
            .or_insert_with(BigRational::zero) += parse_decimal(&settlement.amount)?;
    }
    for ((due_id, component), dates) in timelines {
        let due = s
            .dues
            .iter()
            .find(|d| d.id == due_id)
            .ok_or_else(|| invalid("Échéance de règlement inconnue."))?;
        let maximum = parse_decimal(if component == "rent" {
            &due.rent
        } else {
            &due.charges
        })?;
        let mut paid = BigRational::zero();
        for amount in dates.values() {
            paid += amount;
            if paid.is_negative() || paid > maximum {
                return Err(invalid("Encaissements et règlements non monétaires dépassent une échéance à une date affectée."));
            }
        }
    }
    Ok(())
}

/// All live dependencies are checked; historical term sources may survive a void.
pub fn validate_rentals(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    s: &RentalSnapshot,
) -> Result<()> {
    validate_registry(registry)?;
    validate_properties(registry, properties)?;
    if registry.knowledge_sequence != s.knowledge_sequence
        || properties.knowledge_sequence != s.knowledge_sequence
    {
        return Err(invalid(
            "Les sources locatives n’ont pas la même borne de connaissance.",
        ));
    }
    let records: Vec<(&String, &String)> = s
        .leases
        .iter()
        .map(|r| (&r.id, &r.revision_id))
        .chain(s.lease_terms.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.dues.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.payments.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.allocations.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.expenses.iter().map(|r| (&r.id, &r.revision_id)))
        .collect();
    if records.len() > 20000 {
        return Err(AppError::new(
            "capacity",
            "Le contrôle locatif dépasse le budget de cette version.",
        ));
    }
    let mut ids = HashSet::new();
    let mut revisions = HashSet::new();
    for (id, revision) in records {
        identifier(id)?;
        identifier(revision)?;
        if !ids.insert(id) || !revisions.insert(revision) {
            return Err(invalid(
                "Un objet locatif ou une révision apparaît plusieurs fois.",
            ));
        }
    }
    let entities: HashSet<_> = registry.entities.iter().map(|e| e.id.as_str()).collect();
    let assets: HashSet<_> = registry.assets.iter().map(|e| e.id.as_str()).collect();
    let units: HashMap<_, _> = properties
        .units
        .iter()
        .map(|u| (u.id.as_str(), u))
        .collect();
    let leases: HashMap<_, _> = s.leases.iter().map(|l| (l.id.as_str(), l)).collect();
    for lease in &s.leases {
        let unit = units
            .get(lease.unit_id.as_str())
            .ok_or_else(|| invalid("Le bail doit référencer une unité connue."))?;
        if !entities.contains(lease.tenant_entity_id.as_str()) {
            return Err(invalid("Le locataire du bail est inconnu."));
        }
        validate_interval(&lease.valid_from, lease.valid_to.as_deref())?;
        if !unit.available_for_rent
            || !contained(
                &lease.valid_from,
                lease.valid_to.as_deref(),
                &unit.valid_from,
                unit.valid_to.as_deref(),
            )
        {
            return Err(invalid(
                "Le bail doit rester dans une période de suivi disponible de l’unité.",
            ));
        }
        text(&lease.notes, 10000, false)?;
    }
    disjoint(
        s.leases
            .iter()
            .map(|l| {
                (
                    l.unit_id.as_str(),
                    l.valid_from.as_str(),
                    l.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    let mut historical = HashMap::new();
    for term in &s.lease_term_revisions {
        identifier(&term.id)?;
        identifier(&term.revision_id)?;
        if historical.insert(term.revision_id.as_str(), term).is_some() {
            return Err(invalid(
                "Une source historique de conditions locatives est ambiguë.",
            ));
        }
        if revisions.contains(&term.revision_id) && !s.lease_terms.iter().any(|t| t == term) {
            return Err(invalid(
                "Une source historique de conditions locatives est altérée.",
            ));
        }
    }
    let mut currency_by_lease = HashMap::new();
    for term in &s.lease_terms {
        let lease = leases
            .get(term.lease_id.as_str())
            .ok_or_else(|| invalid("Les conditions concernent un bail inconnu."))?;
        if !contained(
            &term.valid_from,
            term.valid_to.as_deref(),
            &lease.valid_from,
            lease.valid_to.as_deref(),
        ) {
            return Err(invalid(
                "Les conditions doivent rester dans les dates du bail.",
            ));
        }
        if currency_by_lease
            .insert(term.lease_id.as_str(), term.currency.as_str())
            .is_some_and(|c| c != term.currency)
        {
            return Err(invalid("Les périodes d’un bail doivent utiliser la même devise ; créez un nouveau contrat pour changer de devise."));
        }
    }
    for term in s.lease_terms.iter().chain(&s.lease_term_revisions) {
        identifier(&term.lease_id)?;
        validate_interval(&term.valid_from, term.valid_to.as_deref())?;
        nonnegative(&term.rent)?;
        nonnegative(&term.charges)?;
        nonnegative(&term.deposit_required)?;
        currency_minor_units(&term.currency)?;
        if !["monthly", "quarterly", "yearly"].contains(&term.frequency.as_str())
            || term.due_day == 0
            || term.due_day > 31
        {
            return Err(invalid("Fréquence ou jour d’échéance invalide."));
        }
        text(&term.notes, 10000, false)?;
    }
    disjoint(
        s.lease_terms
            .iter()
            .map(|t| {
                (
                    t.lease_id.as_str(),
                    t.valid_from.as_str(),
                    t.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    let mut keys = HashSet::new();
    let dues: HashMap<_, _> = s.dues.iter().map(|d| (d.id.as_str(), d)).collect();
    for due in &s.dues {
        let lease = leases
            .get(due.lease_id.as_str())
            .ok_or_else(|| invalid("L’échéance concerne un bail inconnu."))?;
        let term = s
            .lease_terms
            .iter()
            .chain(&s.lease_term_revisions)
            .find(|t| t.revision_id == due.terms_revision_id && t.lease_id == due.lease_id)
            .ok_or_else(|| {
                invalid(
                    "L’échéance doit référencer une révision exacte des conditions de son bail.",
                )
            })?;
        validate_interval(&due.period_start, Some(&due.period_end))?;
        validate_date(&due.due_on)?;
        if !contained(
            &due.period_start,
            Some(&due.period_end),
            &lease.valid_from,
            lease.valid_to.as_deref(),
        ) || !contained(
            &due.period_start,
            Some(&due.period_end),
            &term.valid_from,
            term.valid_to.as_deref(),
        ) || due.due_on < due.period_start
            || due.due_on >= due.period_end
            || due.currency != term.currency
        {
            return Err(invalid(
                "Dates ou devise de l’échéance incompatibles avec sa source contractuelle.",
            ));
        }
        nonnegative(&due.rent)?;
        nonnegative(&due.charges)?;
        text(&due.generation_key, 400, true)?;
        text(&due.notes, 10000, false)?;
        if !keys.insert(&due.generation_key) {
            return Err(invalid("Une clé d’échéance est déjà utilisée."));
        }
    }
    disjoint(
        s.dues
            .iter()
            .map(|d| {
                (
                    d.lease_id.as_str(),
                    d.period_start.as_str(),
                    Some(d.period_end.as_str()),
                )
            })
            .collect(),
    )?;
    let payments: HashMap<_, _> = s.payments.iter().map(|p| (p.id.as_str(), p)).collect();
    for p in &s.payments {
        if !leases.contains_key(p.lease_id.as_str())
            || !entities.contains(p.recipient_entity_id.as_str())
        {
            return Err(invalid("Le bail ou bénéficiaire du paiement est inconnu."));
        }
        validate_date(&p.effective_on)?;
        currency_minor_units(&p.currency)?;
        text(&p.reference, 1000, false)?;
        text(&p.unlinked_reason, 2000, true)?;
        if !s
            .lease_terms
            .iter()
            .chain(&s.lease_term_revisions)
            .any(|t| t.lease_id == p.lease_id && t.currency == p.currency)
        {
            return Err(invalid(
                "La devise du paiement ne correspond pas au contrat.",
            ));
        }
        let amount = parse_decimal(&p.amount)?;
        match p.kind.as_str() {
            "receipt" if amount > BigRational::zero() && p.original_payment_id.is_none() => {}
            "refund" if amount < BigRational::zero() => {
                let original = p
                    .original_payment_id
                    .as_deref()
                    .and_then(|id| payments.get(id))
                    .ok_or_else(|| {
                        invalid("Le remboursement doit référencer un encaissement d’origine.")
                    })?;
                if original.kind != "receipt"
                    || original.lease_id != p.lease_id
                    || original.currency != p.currency
                    || original.recipient_entity_id != p.recipient_entity_id
                    || p.effective_on < original.effective_on
                {
                    return Err(invalid(
                        "Le remboursement est incompatible avec son encaissement d’origine.",
                    ));
                }
            }
            _ => {
                return Err(invalid(
                    "Un encaissement est positif ; un remboursement négatif et relié à l’original.",
                ))
            }
        }
    }
    let mut per_payment: HashMap<&str, BigRational> = HashMap::new();
    let mut timeline: BTreeMap<&str, Vec<&RentAllocation>> = BTreeMap::new();
    for a in &s.allocations {
        let p = payments
            .get(a.payment_id.as_str())
            .ok_or_else(|| invalid("Le paiement affecté est inconnu."))?;
        let d = dues
            .get(a.due_id.as_str())
            .ok_or_else(|| invalid("L’échéance affectée est inconnue."))?;
        validate_date(&a.effective_on)?;
        timeline.entry(&a.effective_on).or_default().push(a);
        let amount = parse_decimal(&a.amount)?;
        if !["rent", "charges"].contains(&a.component.as_str())
            || a.currency != p.currency
            || a.currency != d.currency
            || p.lease_id != d.lease_id
            || a.effective_on < p.effective_on
            || amount.is_zero()
            || amount.is_negative() != (p.kind == "refund")
        {
            return Err(invalid(
                "Affectation incompatible avec le paiement, l’échéance, la devise ou la date.",
            ));
        }
        *per_payment
            .entry(&a.payment_id)
            .or_insert_with(BigRational::zero) += amount;
    }
    for (id, sum) in per_payment {
        let p = payments[id];
        if sum.abs() > parse_decimal(&p.amount)?.abs() {
            return Err(invalid(
                "Les affectations dépassent le montant du paiement.",
            ));
        }
    }
    let mut running: BTreeMap<(&str, &str), BigRational> = BTreeMap::new();
    for events in timeline.values() {
        let mut touched = BTreeSet::new();
        for a in events {
            let key = (a.due_id.as_str(), a.component.as_str());
            *running.entry(key).or_insert_with(BigRational::zero) += parse_decimal(&a.amount)?;
            touched.insert(key);
        }
        for key in touched {
            let d = dues[key.0];
            let limit = if key.1 == "rent" { &d.rent } else { &d.charges };
            let sum = &running[&key];
            if sum.is_negative() || sum > &parse_decimal(limit)? {
                return Err(invalid("Les affectations dépassent une échéance ou rendent son règlement négatif à une date affectée."));
            }
        }
    }
    // Validate the whole receipt/refund family at each civil date. A refund
    // of an already allocated amount requires its reversal in the same change.
    type BalanceEvent<'a> = (&'a str, Option<(&'a str, &'a str)>, BigRational);
    fn root_of(p: &RentPayment) -> &str {
        if p.kind == "receipt" {
            p.id.as_str()
        } else {
            p.original_payment_id
                .as_deref()
                .expect("validated original receipt")
        }
    }
    let mut balance_dates: BTreeMap<&str, Vec<BalanceEvent<'_>>> = BTreeMap::new();
    for p in &s.payments {
        balance_dates.entry(&p.effective_on).or_default().push((
            root_of(p),
            None,
            parse_decimal(&p.amount)?,
        ));
    }
    for a in &s.allocations {
        balance_dates.entry(&a.effective_on).or_default().push((
            root_of(payments[a.payment_id.as_str()]),
            Some((&a.due_id, &a.component)),
            parse_decimal(&a.amount)?,
        ));
    }
    let mut cash: BTreeMap<&str, BigRational> = BTreeMap::new();
    let mut assigned: BTreeMap<&str, BigRational> = BTreeMap::new();
    let mut components: BTreeMap<(&str, &str, &str), BigRational> = BTreeMap::new();
    for events in balance_dates.values() {
        let mut touched = BTreeSet::new();
        let mut touched_components = BTreeSet::new();
        for (root, component, amount) in events {
            touched.insert(*root);
            if let Some((due, kind)) = component {
                *assigned.entry(root).or_insert_with(BigRational::zero) += amount;
                let key = (*root, *due, *kind);
                *components.entry(key).or_insert_with(BigRational::zero) += amount;
                touched_components.insert(key);
            } else {
                *cash.entry(root).or_insert_with(BigRational::zero) += amount;
            }
        }
        for root in touched {
            let received = cash.get(root).cloned().unwrap_or_else(BigRational::zero);
            let allocated = assigned
                .get(root)
                .cloned()
                .unwrap_or_else(BigRational::zero);
            if received.is_negative() || allocated.is_negative() || allocated > received {
                return Err(invalid("Un remboursement dépasserait l’encaissement disponible ou laisserait ses affectations supérieures au montant net reçu. Reprenez les affectations dans la même opération."));
            }
        }
        for key in touched_components {
            if components[&key].is_negative() {
                return Err(invalid("Un remboursement ne peut reprendre plus que l’affectation de son encaissement d’origine pour cette échéance et cette composante."));
            }
        }
    }
    for expense in &s.expenses {
        if !assets.contains(expense.asset_id.as_str())
            || !properties
                .properties
                .iter()
                .any(|p| p.asset_id == expense.asset_id)
        {
            return Err(invalid("La dépense concerne un bien inconnu."));
        }
        if ![
            "operating",
            "tax",
            "capital_works",
            "interest",
            "principal",
            "financing_fee",
            "other",
        ]
        .contains(&expense.kind.as_str())
        {
            return Err(invalid("La catégorie de dépense locative est inconnue."));
        }
        nonnegative(&expense.amount)?;
        currency_minor_units(&expense.currency)?;
        validate_date(&expense.effective_on)?;
        if let Some(paid) = &expense.paid_on {
            validate_date(paid)?;
        }
        text(&expense.notes, 10000, false)?;
    }
    Ok(())
}

fn covers(mut periods: Vec<(&str, &str)>, start: &str, end: &str) -> bool {
    if start >= end {
        return true;
    }
    periods.sort();
    let mut cursor = start;
    for (from, to) in periods {
        if to <= cursor {
            continue;
        }
        if from > cursor {
            return false;
        }
        cursor = cursor.max(to);
        if cursor >= end {
            return true;
        }
    }
    false
}
fn terms_cover(s: &RentalSnapshot, lease: &str, start: &str, end: &str) -> bool {
    covers(
        s.lease_terms
            .iter()
            .filter(|t| t.lease_id == lease)
            .map(|t| (t.valid_from.as_str(), t.valid_to.as_deref().unwrap_or(end)))
            .collect(),
        start,
        end,
    )
}
fn months(frequency: &str) -> Result<u8> {
    match frequency {
        "monthly" => Ok(1),
        "quarterly" => Ok(3),
        "yearly" => Ok(12),
        _ => Err(invalid("Fréquence locative non prise en charge.")),
    }
}
fn month_start(year: i32, month: u8) -> Result<time::Date> {
    time::Date::from_calendar_date(
        year,
        time::Month::try_from(month).map_err(|_| invalid("Mois invalide."))?,
        1,
    )
    .map_err(|_| invalid("Période hors des dates prises en charge."))
}
fn shift_month(d: time::Date, n: u8) -> Result<time::Date> {
    let absolute = d.year() * 12 + i32::from(u8::from(d.month())) - 1 + i32::from(n);
    month_start(absolute / 12, (absolute % 12 + 1) as u8)
}
fn clipped_day(start: time::Date, day: u8) -> Result<time::Date> {
    let next = shift_month(start, 1)?;
    Ok(start + time::Duration::days(i64::from(day.min((next - start).whole_days() as u8) - 1)))
}

/// Calendar periods are clipped to lease/term boundaries, never to the request.
/// The request selects due dates in [from,to). Prorata uses exact civil days,
/// rounded half-even once to the contractual currency's minor units per due.
pub fn generate_rent_dues(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    s: &RentalSnapshot,
    q: &GenerateRentDuesInput,
) -> Result<Vec<GeneratedRentDue>> {
    validate_rentals(registry, properties, s)?;
    if s.dues
        .len()
        .saturating_mul(s.allocations.len().saturating_add(s.payments.len()))
        > 5_000_000
    {
        return Err(AppError::new(
            "capacity",
            "Le rapport locatif dépasse le budget de calcul de cette version.",
        ));
    }
    validate_interval(&q.from_date, Some(&q.to_date))?;
    if q.expected_sequence != s.knowledge_sequence {
        return Err(AppError::new(
            "conflict",
            "Le registre a changé depuis la préparation des échéances.",
        ));
    }
    text(&q.reason, 2000, true)?;
    let lease = s
        .leases
        .iter()
        .find(|l| l.id == q.lease_id)
        .ok_or_else(|| invalid("Bail inconnu."))?;
    let from = date(&q.from_date)?;
    let to = date(&q.to_date)?;
    if (to - from).whole_days() > 3660 {
        return Err(AppError::new(
            "capacity",
            "Générez au plus dix années d’échéances à la fois.",
        ));
    }
    let requested_start = q.from_date.as_str().max(&lease.valid_from);
    let requested_end = lease
        .valid_to
        .as_deref()
        .unwrap_or(&q.to_date)
        .min(&q.to_date);
    if !terms_cover(s, &lease.id, requested_start, requested_end) {
        return Err(invalid("Des conditions contractuelles manquent dans la période demandée ; aucune échéance supposée n’a été créée."));
    }
    let mut out = Vec::new();
    for term in s.lease_terms.iter().filter(|t| t.lease_id == lease.id) {
        let n = months(&term.frequency)?;
        let first_month = ((u8::from(from.month()) - 1) / n) * n + 1;
        let mut cycle = month_start(from.year(), first_month)?;
        while cycle < to {
            let next = shift_month(cycle, n)?;
            let start = cycle
                .max(date(&lease.valid_from)?)
                .max(date(&term.valid_from)?);
            let mut end = next;
            if let Some(stop) = &lease.valid_to {
                end = end.min(date(stop)?);
            }
            if let Some(stop) = &term.valid_to {
                end = end.min(date(stop)?);
            }
            if start < end {
                let due = clipped_day(cycle, term.due_day)?.max(start).min(
                    end.previous_day()
                        .ok_or_else(|| invalid("Fin de période invalide."))?,
                );
                if due >= from && due < to && due < end {
                    let fraction = BigRational::new(
                        BigInt::from((end - start).whole_days()),
                        BigInt::from((next - cycle).whole_days()),
                    );
                    let rent =
                        canonical_money(&(parse_decimal(&term.rent)? * &fraction), &term.currency)?;
                    let charges = canonical_money(
                        &(parse_decimal(&term.charges)? * &fraction),
                        &term.currency,
                    )?;
                    let period_start = ds(start);
                    let period_end = ds(end);
                    out.push(GeneratedRentDue {lease_id:lease.id.clone(),terms_revision_id:term.revision_id.clone(),generation_key:format!("rent|{}|{}|{}",lease.id,period_start,period_end),period_start,period_end,due_on:ds(due),rent,charges,currency:term.currency.clone(),notes:"Génération calendaire ; prorata jours civils et arrondi à la moitié au pair par échéance.".into()});
                }
            }
            cycle = next;
        }
    }
    out.sort_by(|a, b| {
        (&a.due_on, &a.period_start, &a.generation_key).cmp(&(
            &b.due_on,
            &b.period_start,
            &b.generation_key,
        ))
    });
    Ok(out)
}

/// Domain contract only. Persistence of cash and the deposit obligation is P6.
pub fn deposit_effect(amount: &str) -> Result<DepositEffect> {
    let amount = nonnegative(amount)?;
    Ok(DepositEffect {
        cash_increase: fraction_of(&amount),
        restitution_liability: fraction_of(&amount),
        rental_income: fraction_of(&BigRational::zero()),
    })
}

fn ratio(value: &Fraction) -> Result<BigRational> {
    let n = BigInt::parse_bytes(value.num.as_bytes(), 10)
        .ok_or_else(|| invalid("Numérateur exact invalide."))?;
    let d = BigInt::parse_bytes(value.den.as_bytes(), 10)
        .ok_or_else(|| invalid("Dénominateur exact invalide."))?;
    if d <= BigInt::zero() {
        return Err(invalid("Dénominateur exact invalide."));
    }
    Ok(BigRational::new(n, d))
}
fn warning(out: &mut Vec<RentalWarning>, code: &str, id: &str, message: &str) {
    if !out
        .iter()
        .any(|w| w.code == code && w.object_id == id && w.message == message)
    {
        out.push(RentalWarning {
            code: code.into(),
            object_id: id.into(),
            message: message.into(),
        });
    }
}
fn add_metric(
    totals: &mut BTreeMap<&'static str, Option<BigRational>>,
    key: &'static str,
    value: Option<BigRational>,
) {
    let entry = totals
        .entry(key)
        .or_insert_with(|| Some(BigRational::zero()));
    match (entry.as_mut(), value) {
        (Some(current), Some(value)) => *current += value,
        _ => *entry = None,
    }
}
#[derive(Default)]
struct RentalTrace {
    source_ids: BTreeSet<String>,
    conversions: Vec<FxResult>,
    conversion_keys: BTreeSet<String>,
}
impl RentalTrace {
    fn source(&mut self, id: &str, revision: &str) {
        self.source_ids.extend([id.into(), revision.into()]);
    }
    fn conversion(&mut self, result: &FxResult) {
        for leg in &result.legs {
            self.source(&leg.rate_id, &leg.revision_id);
        }
        let key = format!(
            "{}|{}|{}|{}",
            result.from_currency,
            result.to_currency,
            result.date,
            result
                .legs
                .iter()
                .map(|l| l.revision_id.as_str())
                .collect::<Vec<_>>()
                .join(",")
        );
        if self.conversion_keys.insert(key) {
            self.conversions.push(result.clone());
        }
    }
}
fn fx_value(
    registry: &RegistrySnapshot,
    amount: &BigRational,
    currency: &str,
    as_of: &str,
    id: &str,
    warnings: &mut Vec<RentalWarning>,
    trace: &mut RentalTrace,
) -> Result<Option<BigRational>> {
    let result = crate::fx::convert_currency_unchecked(
        registry,
        &FxQuery {
            amount: "1".into(),
            from_currency: currency.into(),
            to_currency: "EUR".into(),
            date: as_of.into(),
            knowledge_sequence: Some(registry.knowledge_sequence),
        },
    )?;
    for w in &result.warnings {
        warning(warnings, &w.code, id, &w.message);
    }
    trace.conversion(&result);
    result
        .exact_rate
        .as_ref()
        .map(|r| ratio(r).map(|r| amount * r))
        .transpose()
}
fn disposed(properties: &PropertySnapshot, asset: &str, as_of: &str) -> bool {
    properties
        .events
        .iter()
        .any(|e| e.asset_id == asset && e.kind == "disposal" && e.effective_on.as_str() <= as_of)
}
fn weight(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    asset: &str,
    q: &RentalReportRequest,
    as_of: &str,
    warnings: &mut Vec<RentalWarning>,
    trace: &mut RentalTrace,
) -> Result<Option<BigRational>> {
    if q.context != "property" && disposed(properties, asset, as_of) {
        for event in properties.events.iter().filter(|e| {
            e.asset_id == asset && e.kind == "disposal" && e.effective_on.as_str() <= as_of
        }) {
            trace.source(&event.id, &event.revision_id);
        }
        return Ok(Some(BigRational::zero()));
    }
    let w = economic_asset_weight(registry, asset, &q.context, q.person_id.as_deref(), as_of)?;
    trace.source_ids.extend(w.source_ids.iter().cloned());
    if w.unknown {
        warning(warnings,"unknown_economic_rights",asset,"Les droits économiques de cette période sont incomplets ; le montant attribuable reste inconnu.");
        Ok(None)
    } else {
        Ok(Some(ratio(&w.weight)?))
    }
}
fn boundaries(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    start: &str,
    end: &str,
) -> BTreeSet<String> {
    let mut out = BTreeSet::from([start.into(), end.into()]);
    let mut push = |date: &str| {
        if date > start && date < end {
            out.insert(date.into());
        }
    };
    for m in &registry.memberships {
        push(&m.valid_from);
        if let Some(d) = &m.valid_to {
            push(d);
        }
    }
    for p in &registry.participations {
        push(&p.valid_from);
        if let Some(d) = &p.valid_to {
            push(d);
        }
    }
    for o in &registry.ownerships {
        push(&o.valid_from);
        if let Some(d) = &o.valid_to {
            push(d);
        }
    }
    for r in &registry.exchange_rates {
        push(&r.effective_on);
    }
    for e in &properties.events {
        if e.kind == "disposal" {
            push(&e.effective_on);
        }
    }
    out
}
struct AccrualSource<'a> {
    asset: &'a str,
    amount: &'a BigRational,
    currency: &'a str,
    period_start: &'a str,
    period_end: &'a str,
    id: &'a str,
}
fn attributed_period(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    source: AccrualSource<'_>,
    q: &RentalReportRequest,
    warnings: &mut Vec<RentalWarning>,
    trace: &mut RentalTrace,
) -> Result<Option<BigRational>> {
    let AccrualSource {
        asset,
        amount,
        currency,
        period_start,
        period_end,
        id,
    } = source;
    let start = period_start.max(&q.from_date);
    let observed_end = date(&q.date)?
        .next_day()
        .ok_or_else(|| invalid("Date d’observation trop tardive."))?;
    let end = period_end.min(&q.to_date).min(&ds(observed_end)).to_owned();
    if start >= end.as_str() {
        return Ok(Some(BigRational::zero()));
    }
    let denominator = (date(period_end)? - date(period_start)?).whole_days();
    let dates: Vec<_> = boundaries(registry, properties, start, &end)
        .into_iter()
        .collect();
    if dates.len() > 20000 {
        return Err(AppError::new(
            "capacity",
            "Trop de frontières temporelles locatives.",
        ));
    }
    let mut result = BigRational::zero();
    for pair in dates.windows(2) {
        let Some(w) = weight(registry, properties, asset, q, &pair[0], warnings, trace)? else {
            return Ok(None);
        };
        let share = amount
            * &w
            * BigRational::new(
                BigInt::from((date(&pair[1])? - date(&pair[0])?).whole_days()),
                BigInt::from(denominator),
            );
        if share.is_zero() {
            continue;
        }
        let Some(eur) = fx_value(registry, &share, currency, &pair[0], id, warnings, trace)? else {
            return Ok(None);
        };
        result += eur;
    }
    Ok(Some(result))
}
fn cash_scope(
    registry: &RegistrySnapshot,
    q: &RentalReportRequest,
    recipient: &str,
    as_of: &str,
    warnings: &mut Vec<RentalWarning>,
    trace: &mut RentalTrace,
) -> bool {
    if let Some(entity) = registry.entities.iter().find(|e| e.id == recipient) {
        trace.source(&entity.id, &entity.revision_id);
    }
    for membership in registry.memberships.iter().filter(|m| {
        m.person_id == recipient && applies(&m.valid_from, m.valid_to.as_deref(), as_of)
    }) {
        trace.source(&membership.id, &membership.revision_id);
    }
    if q.context == "property" {
        return true;
    }
    if q.context == "person" || q.context == "company" {
        if q.context == "person"
            && registry
                .entities
                .iter()
                .any(|e| e.id == recipient && e.kind == "company")
        {
            warning(warnings,"company_cash_not_distributed",recipient,"Un encaissement reçu par une société n’est pas présenté comme une somme reçue personnellement par ses associés.");
        }
        return q.person_id.as_deref() == Some(recipient);
    }
    let member = registry
        .memberships
        .iter()
        .any(|m| m.person_id == recipient && applies(&m.valid_from, m.valid_to.as_deref(), as_of));
    if !member
        && registry
            .entities
            .iter()
            .any(|e| e.id == recipient && e.kind == "company")
    {
        warning(warnings,"company_cash_not_distributed",recipient,"Un encaissement reçu par une société n’est pas présenté comme une somme reçue personnellement par ses associés.");
    }
    member
}
fn report_due(
    s: &RentalSnapshot,
    settlements: &[NonCashRentSettlement],
    due: &RentDue,
    asset: &str,
    as_of: &str,
) -> Result<RentDueStatus> {
    let rent = allocated_for_report(s, settlements, &due.id, "rent", as_of)?;
    let charges = allocated_for_report(s, settlements, &due.id, "charges", as_of)?;
    let remaining_rent = parse_decimal(&due.rent)? - &rent;
    let remaining_charges = parse_decimal(&due.charges)? - &charges;
    let mut events: Vec<_> = s
        .allocations
        .iter()
        .filter(|a| a.due_id == due.id && a.effective_on.as_str() <= as_of)
        .collect();
    events.sort_by(|a, b| (&a.effective_on, &a.id).cmp(&(&b.effective_on, &b.id)));
    let mut dated_amounts: Vec<(&str, &str, BigRational)> = events
        .iter()
        .map(|a| {
            Ok((
                a.effective_on.as_str(),
                a.id.as_str(),
                parse_decimal(&a.amount)?,
            ))
        })
        .collect::<Result<_>>()?;
    for v in settlements
        .iter()
        .filter(|v| v.due_id == due.id && v.effective_on.as_str() <= as_of)
    {
        dated_amounts.push((
            &v.effective_on,
            v.source_ids
                .first()
                .map(String::as_str)
                .unwrap_or("settlement"),
            parse_decimal(&v.amount)?,
        ));
    }
    dated_amounts.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)));
    let mut sum = BigRational::zero();
    let total = parse_decimal(&due.rent)? + parse_decimal(&due.charges)?;
    let mut settled = None;
    let mut late = false;
    for (effective_on, _, amount) in &dated_amounts {
        sum += amount;
        if sum >= total {
            settled = Some((*effective_on).to_owned());
        } else {
            settled = None;
        }
        if *effective_on > due.due_on.as_str() && amount > &BigRational::zero() {
            late = true;
        }
    }
    let overdue =
        due.due_on.as_str() < as_of && (&remaining_rent + &remaining_charges) > BigRational::zero();
    late |= overdue;
    let mut sources = BTreeSet::from([
        due.id.clone(),
        due.revision_id.clone(),
        due.terms_revision_id.clone(),
    ]);
    for a in events {
        sources.extend([a.id.clone(), a.revision_id.clone(), a.payment_id.clone()]);
        if let Some(payment) = s.payments.iter().find(|p| p.id == a.payment_id) {
            sources.insert(payment.revision_id.clone());
        }
    }
    for settlement in settlements
        .iter()
        .filter(|v| v.due_id == due.id && v.effective_on.as_str() <= as_of)
    {
        sources.extend(settlement.source_ids.iter().cloned());
    }
    Ok(RentDueStatus {
        due_id: due.id.clone(),
        lease_id: due.lease_id.clone(),
        asset_id: asset.into(),
        due_on: due.due_on.clone(),
        period_start: due.period_start.clone(),
        period_end: due.period_end.clone(),
        currency: due.currency.clone(),
        rent: due.rent.clone(),
        charges: due.charges.clone(),
        allocated_rent: canonical_money(&rent, &due.currency)?,
        allocated_charges: canonical_money(&charges, &due.currency)?,
        outstanding_rent: canonical_money(&remaining_rent, &due.currency)?,
        outstanding_charges: canonical_money(&remaining_charges, &due.currency)?,
        overdue_days: if overdue {
            (date(as_of)? - date(&due.due_on)?).whole_days()
        } else {
            0
        },
        settled_on: settled,
        historical_late: late,
        source_ids: sources.into_iter().collect(),
    })
}

/// Analytical property revenue is allocated when it accrues. Receipt totals
/// use the explicitly named recipient on the actual receipt date. Neither is
/// inserted into the asset stock or represented as a second bank movement.
pub fn rental_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    s: &RentalSnapshot,
    q: &RentalReportRequest,
) -> Result<RentalReport> {
    rental_report_with_settlements(registry, properties, s, q, &[])
}

/// Deposits applied to a due change its settlement and residual, not the
/// payment receipts or the NOI already accrued from the contractual period.
pub fn rental_report_with_settlements(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    s: &RentalSnapshot,
    q: &RentalReportRequest,
    settlements: &[NonCashRentSettlement],
) -> Result<RentalReport> {
    validate_rentals(registry, properties, s)?;
    validate_non_cash_settlements(s, settlements)?;
    if s.dues
        .len()
        .saturating_mul(s.allocations.len().saturating_add(s.payments.len()))
        > 5_000_000
    {
        return Err(AppError::new(
            "capacity",
            "Le rapport locatif dépasse le budget de calcul de cette version.",
        ));
    }
    validate_interval(&q.from_date, Some(&q.to_date))?;
    validate_date(&q.date)?;
    if !["family", "person", "company", "property"].contains(&q.context.as_str())
        || (["person", "company"].contains(&q.context.as_str())
            && !registry
                .entities
                .iter()
                .any(|e| e.kind == q.context && Some(&e.id) == q.person_id.as_ref()))
        || (!["person", "company"].contains(&q.context.as_str()) && q.person_id.is_some())
    {
        return Err(invalid("Périmètre locatif invalide."));
    }
    if q.knowledge_sequence
        .is_some_and(|k| k != s.knowledge_sequence)
    {
        return Err(invalid(
            "La borne de connaissance ne correspond pas aux sources locatives.",
        ));
    }
    if q.asset_id
        .as_ref()
        .is_some_and(|asset| !properties.properties.iter().any(|p| &p.asset_id == asset))
    {
        return Err(invalid("Le filtre immobilier est inconnu."));
    }
    let mut trace = RentalTrace::default();
    let mut warnings = Vec::new();
    warning(&mut warnings,"treasury_not_integrated","scope","Les paiements sont des faits locatifs non rapprochés ; ils ne créent aucune écriture bancaire. Le cash-flow après financement et les dépôts persistés restent à intégrer en phase 6.");
    warning(&mut warnings,"occupation_contractual","scope","L’occupation est calculée depuis les périodes de bail et de disponibilité déclarées, en jours-unités ; elle n’est pas une mesure de recouvrement ni une constatation physique.");
    warning(&mut warnings,"noi_annualization","scope","Le rendement net annualise le revenu d’exploitation de la période sur une convention de 365 jours ; les travaux en capital et le financement sont exclus du NOI.");
    let units: HashMap<_, _> = properties
        .units
        .iter()
        .map(|u| (u.id.as_str(), u))
        .collect();
    let lease_asset: HashMap<_, _> = s
        .leases
        .iter()
        .filter_map(|l| {
            units
                .get(l.unit_id.as_str())
                .map(|u| (l.id.as_str(), u.property_asset_id.as_str()))
        })
        .collect();
    if q.country
        .as_ref()
        .is_some_and(|country| !["FR", "TN", "CH", "AE"].contains(&country.as_str()))
    {
        return Err(invalid("Pays de filtrage locatif non pris en charge."));
    }
    let dated_registry = apply_profile_periods(registry, &q.date)?;
    let selected = |asset: &str| {
        q.asset_id.as_deref().is_none_or(|id| id == asset)
            && q.country.as_ref().is_none_or(|country| {
                crate::profiles::profile_is_dated(registry, "asset", asset, &q.date)
                    && dated_registry
                        .assets
                        .iter()
                        .any(|a| a.id == asset && &a.country == country)
            })
    };
    if let Some(country) = &q.country {
        if properties
            .properties
            .iter()
            .any(|p| !crate::profiles::profile_is_dated(registry, "asset", &p.asset_id, &q.date))
        {
            warning(&mut warnings,"undated_country_excluded",country,"Le filtre par pays exclut les biens sans profil descriptif daté à la date du rapport, sans déduire leur pays historique.");
        }
    }

    let in_period = |d: &str| d >= q.from_date.as_str() && d < q.to_date.as_str();
    let mut totals: BTreeMap<&'static str, Option<BigRational>> = [
        "contractual_annual_rent",
        "rent_due",
        "charges_due",
        "rent_received",
        "charges_received",
        "outstanding_rent",
        "outstanding_charges",
        "unallocated_payments",
        "economic_rent",
        "economic_charges",
        "operating_expenses",
    ]
    .into_iter()
    .map(|k| (k, Some(BigRational::zero())))
    .collect();
    let mut due_statuses = Vec::new();
    for due in &s.dues {
        let asset = lease_asset[due.lease_id.as_str()];
        if !selected(asset) {
            continue;
        }
        if due.period_start <= q.date {
            trace.source(&due.id, &due.revision_id);
            trace.source_ids.insert(due.terms_revision_id.clone());
            if let Some(term) = s
                .lease_terms
                .iter()
                .chain(&s.lease_term_revisions)
                .find(|t| t.revision_id == due.terms_revision_id)
            {
                trace.source(&term.id, &term.revision_id);
            }
        }
        if due.due_on <= q.date {
            due_statuses.push(report_due(s, settlements, due, asset, &q.date)?);
            // Dues and outstanding retain the creditor weights of their accrued
            // period. A disposal cannot transfer old arrears to a new owner.
            let mut historical_q = q.clone();
            historical_q.from_date = due.period_start.clone();
            historical_q.to_date = due.period_end.clone();
            historical_q.date = ds(date(&due.period_end)?
                .previous_day()
                .ok_or_else(|| invalid("Période invalide."))?);
            for (component, source, due_key, outstanding_key) in [
                ("rent", &due.rent, "rent_due", "outstanding_rent"),
                (
                    "charges",
                    &due.charges,
                    "charges_due",
                    "outstanding_charges",
                ),
            ] {
                let source = parse_decimal(source)?;
                let outstanding =
                    &source - allocated_for_report(s, settlements, &due.id, component, &q.date)?;
                // Outstanding is a stock at D: apply D's FX, while preserving
                // the historical creditor share derived above.
                let outstanding_eur = if due.currency == "EUR" {
                    attributed_period(
                        registry,
                        properties,
                        AccrualSource {
                            asset,
                            amount: &outstanding,
                            currency: "EUR",
                            period_start: &due.period_start,
                            period_end: &due.period_end,
                            id: &due.id,
                        },
                        &historical_q,
                        &mut warnings,
                        &mut trace,
                    )?
                } else {
                    let historical_weight = attributed_period(
                        registry,
                        properties,
                        AccrualSource {
                            asset,
                            amount: &BigRational::from_integer(BigInt::from(1)),
                            currency: "EUR",
                            period_start: &due.period_start,
                            period_end: &due.period_end,
                            id: &due.id,
                        },
                        &historical_q,
                        &mut warnings,
                        &mut trace,
                    )?;
                    match historical_weight {
                        Some(w) => fx_value(
                            registry,
                            &(&outstanding * w),
                            &due.currency,
                            &q.date,
                            &due.id,
                            &mut warnings,
                            &mut trace,
                        )?,
                        None => None,
                    }
                };
                add_metric(&mut totals, outstanding_key, outstanding_eur);
                if in_period(&due.due_on) {
                    let share = attributed_period(
                        registry,
                        properties,
                        AccrualSource {
                            asset,
                            amount: &BigRational::from_integer(BigInt::from(1)),
                            currency: "EUR",
                            period_start: &due.period_start,
                            period_end: &due.period_end,
                            id: &due.id,
                        },
                        &historical_q,
                        &mut warnings,
                        &mut trace,
                    )?;
                    add_metric(
                        &mut totals,
                        due_key,
                        match share {
                            Some(w) => fx_value(
                                registry,
                                &(&source * w),
                                &due.currency,
                                &due.due_on,
                                &due.id,
                                &mut warnings,
                                &mut trace,
                            )?,
                            None => None,
                        },
                    );
                }
            }
        }
        for (source, key) in [
            (&due.rent, "economic_rent"),
            (&due.charges, "economic_charges"),
        ] {
            let value = attributed_period(
                registry,
                properties,
                AccrualSource {
                    asset,
                    amount: &parse_decimal(source)?,
                    currency: &due.currency,
                    period_start: &due.period_start,
                    period_end: &due.period_end,
                    id: &due.id,
                },
                q,
                &mut warnings,
                &mut trace,
            )?;
            add_metric(&mut totals, key, value);
        }
    }
    for p in &s.payments {
        let asset = lease_asset[p.lease_id.as_str()];
        if !selected(asset)
            || p.effective_on > q.date
            || !in_period(&p.effective_on)
            || !cash_scope(
                registry,
                q,
                &p.recipient_entity_id,
                &p.effective_on,
                &mut warnings,
                &mut trace,
            )
        {
            continue;
        }
        trace.source(&p.id, &p.revision_id);
        trace.source_ids.insert(p.recipient_entity_id.clone());
        for a in s
            .allocations
            .iter()
            .filter(|a| a.payment_id == p.id && a.effective_on <= q.date)
        {
            trace.source(&a.id, &a.revision_id);
        }
        let mut allocated_total = BigRational::zero();
        for component in ["rent", "charges"] {
            let amount = s
                .allocations
                .iter()
                .filter(|a| {
                    a.payment_id == p.id && a.component == component && a.effective_on <= q.date
                })
                .try_fold(BigRational::zero(), |sum, a| {
                    Ok(sum + parse_decimal(&a.amount)?)
                })?;
            allocated_total += &amount;
            let value = fx_value(
                registry,
                &amount,
                &p.currency,
                &p.effective_on,
                &p.id,
                &mut warnings,
                &mut trace,
            )?;
            add_metric(
                &mut totals,
                if component == "rent" {
                    "rent_received"
                } else {
                    "charges_received"
                },
                value,
            );
        }
        add_metric(
            &mut totals,
            "unallocated_payments",
            fx_value(
                registry,
                &(parse_decimal(&p.amount)? - allocated_total),
                &p.currency,
                &p.effective_on,
                &p.id,
                &mut warnings,
                &mut trace,
            )?,
        );
    }
    for term in s
        .lease_terms
        .iter()
        .filter(|t| applies(&t.valid_from, t.valid_to.as_deref(), &q.date))
    {
        let asset = lease_asset[term.lease_id.as_str()];
        let lease = s
            .leases
            .iter()
            .find(|l| l.id == term.lease_id)
            .expect("validated lease");
        if !selected(asset) || !applies(&lease.valid_from, lease.valid_to.as_deref(), &q.date) {
            continue;
        }
        trace.source(&term.id, &term.revision_id);
        trace.source(&lease.id, &lease.revision_id);
        let annual = parse_decimal(&term.rent)? * BigInt::from(12 / months(&term.frequency)?);
        let value = match weight(
            registry,
            properties,
            asset,
            q,
            &q.date,
            &mut warnings,
            &mut trace,
        )? {
            Some(w) => fx_value(
                registry,
                &(annual * w),
                &term.currency,
                &q.date,
                &term.id,
                &mut warnings,
                &mut trace,
            )?,
            None => None,
        };
        add_metric(&mut totals, "contractual_annual_rent", value);
    }
    for expense in &s.expenses {
        if !selected(&expense.asset_id)
            || !in_period(&expense.effective_on)
            || expense.effective_on > q.date
            || !["operating", "tax"].contains(&expense.kind.as_str())
        {
            continue;
        }
        trace.source(&expense.id, &expense.revision_id);
        let value = match weight(
            registry,
            properties,
            &expense.asset_id,
            q,
            &expense.effective_on,
            &mut warnings,
            &mut trace,
        )? {
            Some(w) => fx_value(
                registry,
                &(parse_decimal(&expense.amount)? * w),
                &expense.currency,
                &expense.effective_on,
                &expense.id,
                &mut warnings,
                &mut trace,
            )?,
            None => None,
        };
        add_metric(&mut totals, "operating_expenses", value);
    }
    // Missing contractual data only affects the scope exposed to that period.
    // Cash receipts retain their explicitly named recipient independently.
    let observed_stop = ds(date(&q.date)?
        .next_day()
        .ok_or_else(|| invalid("Date d’observation hors limite."))?);
    for lease in &s.leases {
        let asset = lease_asset[lease.id.as_str()];
        if !selected(asset) {
            continue;
        }
        if applies(&lease.valid_from, lease.valid_to.as_deref(), &q.date)
            && !s.lease_terms.iter().any(|t| {
                t.lease_id == lease.id && applies(&t.valid_from, t.valid_to.as_deref(), &q.date)
            })
            && weight(
                registry,
                properties,
                asset,
                q,
                &q.date,
                &mut warnings,
                &mut trace,
            )?
            .is_none_or(|w| !w.is_zero())
        {
            totals.insert("contractual_annual_rent", None);
            warning(
                &mut warnings,
                "missing_lease_terms",
                &lease.id,
                "Le bail actif n’a pas de conditions applicables ; le loyer attendu reste inconnu.",
            );
        }
        let start = lease.valid_from.as_str().max(&q.from_date);
        let end = lease
            .valid_to
            .as_deref()
            .unwrap_or(&q.to_date)
            .min(&q.to_date)
            .min(&observed_stop);
        if start >= end || (q.context != "property" && disposed(properties, asset, start)) {
            continue;
        }
        let mut dates = boundaries(registry, properties, start, end);
        for (from, to) in s
            .lease_terms
            .iter()
            .filter(|t| t.lease_id == lease.id)
            .map(|t| (t.valid_from.as_str(), t.valid_to.as_deref().unwrap_or(end)))
            .chain(
                s.dues
                    .iter()
                    .filter(|d| d.lease_id == lease.id)
                    .map(|d| (d.period_start.as_str(), d.period_end.as_str())),
            )
        {
            for date in [from, to] {
                if date > start && date < end {
                    dates.insert(date.into());
                }
            }
        }
        if dates.len() > 20000 {
            return Err(AppError::new(
                "capacity",
                "Trop de frontières temporelles locatives.",
            ));
        }
        let dates: Vec<_> = dates.into_iter().collect();
        let mut missing_terms = false;
        let mut missing_dues = false;
        for pair in dates.windows(2) {
            let terms_known = terms_cover(s, &lease.id, &pair[0], &pair[1]);
            let dues_known = covers(
                s.dues
                    .iter()
                    .filter(|d| d.lease_id == lease.id)
                    .map(|d| (d.period_start.as_str(), d.period_end.as_str()))
                    .collect(),
                &pair[0],
                &pair[1],
            );
            if terms_known && dues_known {
                continue;
            }
            if weight(
                registry,
                properties,
                asset,
                q,
                &pair[0],
                &mut warnings,
                &mut trace,
            )?
            .is_some_and(|w| w.is_zero())
            {
                continue;
            }
            missing_terms |= !terms_known;
            missing_dues |= !dues_known;
        }
        if missing_terms || missing_dues {
            for key in [
                "economic_rent",
                "economic_charges",
                "rent_due",
                "charges_due",
                "outstanding_rent",
                "outstanding_charges",
            ] {
                totals.insert(key, None);
            }
            warning(&mut warnings,if missing_terms{"missing_lease_terms"}else{"missing_rent_dues"},&lease.id,"La période courue du bail exposée au périmètre n’est pas entièrement couverte par des conditions et échéances enregistrées ; le revenu et l’impayé complets restent inconnus.");
        }
    }
    let noi = match (
        &totals["economic_rent"],
        &totals["economic_charges"],
        &totals["operating_expenses"],
    ) {
        (Some(r), Some(c), Some(e)) => Some(r + c - e),
        _ => None,
    };
    totals.insert("noi", noi.clone());
    let mut reference = Some(BigRational::zero());
    let asset_ids: BTreeSet<_> = properties
        .units
        .iter()
        .filter(|u| selected(&u.property_asset_id))
        .map(|u| u.property_asset_id.as_str())
        .collect();
    for asset in asset_ids {
        if disposed(properties, asset, &q.date) {
            continue;
        }
        let Some(w) = weight(
            registry,
            properties,
            asset,
            q,
            &q.date,
            &mut warnings,
            &mut trace,
        )?
        else {
            reference = None;
            continue;
        };
        if w.is_zero() {
            continue;
        }
        let value = registry
            .valuations
            .iter()
            .filter(|v| {
                v.target_kind == "asset" && v.target_id == asset && v.effective_on <= q.date
            })
            .max_by_key(|v| &v.effective_on);
        if let Some(v) = value {
            trace.source(&v.id, &v.revision_id);
        }
        let converted = match value {
            Some(v) => fx_value(
                registry,
                &(parse_decimal(&v.amount)? * w),
                &v.currency,
                &q.date,
                asset,
                &mut warnings,
                &mut trace,
            )?,
            None => {
                warning(&mut warnings,"missing_valuation",asset,"Le rendement n’est pas calculable sans estimation applicable du bien ; le coût d’acquisition n’est pas repris par défaut.");
                None
            }
        };
        match (&mut reference, converted) {
            (Some(total), Some(n)) => *total += n,
            _ => reference = None,
        }
    }
    let observed_end_for_yield = ds(date(&q.date)?
        .next_day()
        .ok_or_else(|| invalid("Date d’observation hors limite."))?);
    let observed_to = q.to_date.as_str().min(&observed_end_for_yield);
    let days = (date(observed_to)? - date(&q.from_date)?)
        .whole_days()
        .max(0);
    warning(&mut warnings,"observed_yield_period","scope",&format!("Le NOI et son annualisation portent sur les jours observés du {} au {} exclu ({} jour(s)), avec une convention de 365 jours ; une borne de fin future n’ajoute aucun jour au dénominateur.",q.from_date,observed_to,days));
    let yield_ratio =
        |numerator: &Option<BigRational>, annualize: bool| match (numerator, &reference) {
            (Some(n), Some(d)) if d > &BigRational::zero() && (!annualize || days > 0) => Some(
                n / d
                    * BigInt::from(100)
                    * if annualize {
                        BigRational::new(BigInt::from(365), BigInt::from(days))
                    } else {
                        BigRational::from_integer(BigInt::from(1))
                    },
            ),
            _ => None,
        };
    let gross_yield = yield_ratio(&totals["contractual_annual_rent"], false);
    let net_yield = yield_ratio(&noi, true);
    let mut available = 0_u64;
    let mut occupied = 0_u64;
    let mut occupation_unknown = false;
    let observed_end = ds(date(&q.date)?
        .next_day()
        .ok_or_else(|| invalid("Date d’observation hors limite."))?);
    for unit in properties
        .units
        .iter()
        .filter(|u| selected(&u.property_asset_id) && u.available_for_rent)
    {
        let start = unit.valid_from.as_str().max(&q.from_date);
        let end = unit
            .valid_to
            .as_deref()
            .unwrap_or(&q.to_date)
            .min(&q.to_date)
            .min(&observed_end);
        if start >= end {
            continue;
        }
        let dates: Vec<_> = boundaries(registry, properties, start, end)
            .into_iter()
            .collect();
        for pair in dates.windows(2) {
            let Some(w) = weight(
                registry,
                properties,
                &unit.property_asset_id,
                q,
                &pair[0],
                &mut warnings,
                &mut trace,
            )?
            else {
                occupation_unknown = true;
                continue;
            };
            if w.is_zero() {
                continue;
            }
            // Physical contractual occupation counts each relevant unit once,
            // rather than multiplying days by the financial ownership share.
            trace.source(&unit.id, &unit.revision_id);
            available += (date(&pair[1])? - date(&pair[0])?).whole_days() as u64;
            for lease in s.leases.iter().filter(|l| l.unit_id == unit.id) {
                let lstart = lease.valid_from.as_str().max(&pair[0]);
                let lend = lease.valid_to.as_deref().unwrap_or(&pair[1]).min(&pair[1]);
                if lstart < lend {
                    trace.source(&lease.id, &lease.revision_id);
                    occupied += (date(lend)? - date(lstart)?).whole_days() as u64;
                }
            }
        }
    }
    if occupation_unknown {
        warning(&mut warnings,"occupation_scope_unknown","scope","Des droits ou appartenances manquent pour déterminer les jours-unités du périmètre ; seuls les jours connus sont détaillés et le taux d’occupation reste non calculable.");
    }
    let occupancy = (available > 0 && !occupation_unknown).then(|| {
        format_eur(&BigRational::new(
            BigInt::from(occupied * 100),
            BigInt::from(available),
        ))
    });
    let display = |key: &str| totals.get(key).and_then(|n| n.as_ref()).map(format_eur);
    for due in &due_statuses {
        trace.source_ids.extend(due.source_ids.iter().cloned());
    }
    let exact_totals = totals
        .iter()
        .filter_map(|(k, v)| v.as_ref().map(|n| (k.to_string(), fraction_of(n))))
        .collect();
    due_statuses.sort_by(|a, b| (&a.due_on, &a.due_id).cmp(&(&b.due_on, &b.due_id)));
    Ok(RentalReport {
        source_ids: trace.source_ids.into_iter().collect(),
        conversions: trace.conversions,
        from_date: q.from_date.clone(),
        to_date: q.to_date.clone(),
        date: q.date.clone(),
        knowledge_sequence: s.knowledge_sequence,
        status: if totals.values().any(Option::is_none) {
            "partial"
        } else {
            "complete"
        }
        .into(),
        contractual_annual_rent_eur: display("contractual_annual_rent"),
        rent_due_eur: display("rent_due"),
        charges_due_eur: display("charges_due"),
        rent_received_eur: display("rent_received"),
        charges_received_eur: display("charges_received"),
        outstanding_rent_eur: display("outstanding_rent"),
        outstanding_charges_eur: display("outstanding_charges"),
        unallocated_payments_eur: display("unallocated_payments"),
        economic_rent_eur: display("economic_rent"),
        operating_expenses_eur: display("operating_expenses"),
        noi_eur: display("noi"),
        gross_yield_percent: gross_yield.as_ref().map(format_eur),
        net_yield_percent: net_yield.as_ref().map(format_eur),
        cash_flow_after_financing_eur: None,
        exact_totals,
        occupation: RentalOccupation {
            occupied_unit_days: occupied,
            available_unit_days: available,
            percent: occupancy,
            basis: "contractual".into(),
        },
        dues: due_statuses,
        warnings,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundRentAllocationInput {
    pub due_id: String,
    pub component: String,
    pub amount: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefundRentPaymentInput {
    pub expected_sequence: u64,
    pub payment: SaveRentPaymentInput,
    pub allocations: Vec<RefundRentAllocationInput>,
}
