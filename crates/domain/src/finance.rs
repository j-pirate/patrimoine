//! Consolidated positions from the recorded inventory and its explicit coverage.
use crate::*;
use serde::{Deserialize, Serialize};

macro_rules! finance_record {
    ($row:ident,$input:ident,{$($field:ident:$ty:ty),*$(,)?}) => {
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row {pub id:String,pub revision_id:String,$(pub $field:$ty,)*}
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input {pub id:Option<String>,pub expected_revision_id:Option<String>,pub reason:String,$(pub $field:$ty,)*}
    }
}
finance_record!(FinanceCoverage,SaveFinanceCoverageInput,{
    valid_from:String,valid_to:Option<String>,assets_complete:bool,debts_complete:bool,
    rentals_complete:bool,notes:String,
});
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RentClaimParty {
    pub entity_id: String,
    pub economic_share: Option<Fraction>,
}
finance_record!(RentClaim,SaveRentClaimInput,{
    due_id:String,parties:Vec<RentClaimParty>,valid_from:String,valid_to:Option<String>,
    represented_by_debt_id:Option<String>,notes:String,
});
finance_record!(RentClaimAdjustment,SaveRentClaimAdjustmentInput,{
    due_id:String,creditor_entity_id:String,kind:String,amount:String,effective_on:String,
    source:String,notes:String,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinanceSnapshot {
    pub coverages: Vec<FinanceCoverage>,
    pub rent_claims: Vec<RentClaim>,
    pub rent_claim_adjustments: Vec<RentClaimAdjustment>,
    pub knowledge_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinancialReportRequest {
    pub context: String,
    pub person_id: Option<String>,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialPosition {
    pub object_id: String,
    pub label: String,
    pub kind: String,
    pub entity_id: Option<String>,
    pub country: Option<String>,
    pub currency: String,
    pub amount_original: Option<String>,
    pub exact_original: Option<Fraction>,
    pub weight: Option<Fraction>,
    pub amount_eur: Option<String>,
    pub exact_eur: Option<Fraction>,
    pub source_ids: Vec<String>,
    pub conversion: Option<FxResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialElimination {
    pub object_id: String,
    pub label: String,
    pub creditor_before_eur: String,
    pub debtor_before_eur: String,
    pub eliminated_eur: String,
    pub creditor_after_eur: String,
    pub debtor_after_eur: String,
    pub exact_eliminated: Fraction,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinancialReport {
    pub context: String,
    pub person_id: Option<String>,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub coverage_confirmed: bool,
    pub assets_eur: Option<String>,
    pub liabilities_eur: Option<String>,
    pub net_eur: Option<String>,
    pub exact_assets: Option<Fraction>,
    pub exact_liabilities: Option<Fraction>,
    pub exact_net: Option<Fraction>,
    pub known_assets_eur: String,
    pub known_liabilities_eur: String,
    pub known_net_eur: String,
    pub positions: Vec<FinancialPosition>,
    pub eliminations: Vec<FinancialElimination>,
    pub source_ids: Vec<String>,
    pub warnings: Vec<FinancialWarning>,
}

use crate::exact::{format_currency, format_eur, fraction_of, parse_decimal, parse_fraction};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeMap, BTreeSet, HashSet};

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn exact(f: &Fraction) -> Result<BigRational> {
    let n = f
        .num
        .parse::<BigInt>()
        .map_err(|_| invalid("Numérateur financier invalide."))?;
    let d = f
        .den
        .parse::<BigInt>()
        .map_err(|_| invalid("Dénominateur financier invalide."))?;
    if d <= BigInt::zero() {
        return Err(invalid("Dénominateur financier non positif."));
    }
    Ok(BigRational::new(n, d))
}
fn identifier(s: &str) -> Result<()> {
    if s.is_empty()
        || s.len() > 128
        || !s
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("Un identifiant financier est invalide."));
    }
    Ok(())
}
fn text(s: &str, maximum: usize, required: bool) -> Result<()> {
    if s.chars().count() > maximum
        || (required && s.trim().is_empty())
        || s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid(
            "Un texte financier est vide, trop long ou invalide.",
        ));
    }
    Ok(())
}

struct View<'a> {
    registry: &'a RegistrySnapshot,
    properties: &'a PropertySnapshot,
    rentals: &'a RentalSnapshot,
    treasury: &'a TreasurySnapshot,
    debts: &'a DebtSnapshot,
    finance: &'a FinanceSnapshot,
}
impl View<'_> {
    fn active_claim(&self, due: &str, date: &str) -> Option<&RentClaim> {
        self.finance
            .rent_claims
            .iter()
            .find(|c| c.due_id == due && applies(&c.valid_from, c.valid_to.as_deref(), date))
    }
    fn settled_cash(&self, due: &str, component: &str, date: &str) -> Result<BigRational> {
        self.rentals
            .allocations
            .iter()
            .filter(|a| {
                a.due_id == due && a.component == component && a.effective_on.as_str() <= date
            })
            .try_fold(BigRational::zero(), |sum, a| {
                Ok(sum + parse_decimal(&a.amount)?)
            })
    }
    fn applied(
        &self,
        due: &str,
        component: &str,
        holder: Option<&str>,
        date: &str,
    ) -> Result<BigRational> {
        self.treasury
            .deposits
            .iter()
            .filter(|m| {
                m.kind == "applied_to_due"
                    && m.due_id.as_deref() == Some(due)
                    && m.component == component
                    && m.effective_on.as_str() <= date
                    && holder.is_none_or(|h| h == m.recipient_entity_id)
            })
            .try_fold(BigRational::zero(), |sum, m| {
                Ok(sum - parse_decimal(&m.amount)?)
            })
    }
    fn remaining(&self, due: &RentDue, component: &str, date: &str) -> Result<BigRational> {
        let initial = if component == "rent" {
            &due.rent
        } else {
            &due.charges
        };
        Ok(parse_decimal(initial)?
            - self.settled_cash(&due.id, component, date)?
            - self.applied(&due.id, component, None, date)?)
    }
    fn party_remaining(
        &self,
        due: &RentDue,
        claim: &RentClaim,
        entity: &str,
        date: &str,
    ) -> Result<Option<BigRational>> {
        let _ = claim;
        match (
            self.party_components(due, "rent", date)?
                .get(entity)
                .cloned()
                .flatten(),
            self.party_components(due, "charges", date)?
                .get(entity)
                .cloned()
                .flatten(),
        ) {
            (Some(r), Some(c)) => Ok(Some(r + c)),
            _ => Ok(None),
        }
    }
    fn party_components(
        &self,
        due: &RentDue,
        component: &str,
        at: &str,
    ) -> Result<BTreeMap<String, Option<BigRational>>> {
        let mut dates: BTreeSet<&str> = self
            .finance
            .rent_claims
            .iter()
            .filter(|c| c.due_id == due.id && c.valid_from.as_str() <= at)
            .map(|c| c.valid_from.as_str())
            .collect();
        dates.extend(
            self.rentals
                .allocations
                .iter()
                .filter(|a| {
                    a.due_id == due.id && a.component == component && a.effective_on.as_str() <= at
                })
                .map(|a| a.effective_on.as_str()),
        );
        dates.extend(
            self.treasury
                .deposits
                .iter()
                .filter(|m| {
                    m.kind == "applied_to_due"
                        && m.due_id.as_deref() == Some(&due.id)
                        && m.component == component
                        && m.effective_on.as_str() <= at
                })
                .map(|m| m.effective_on.as_str()),
        );
        let mut outstanding = parse_decimal(if component == "rent" {
            &due.rent
        } else {
            &due.charges
        })?;
        let mut balances = BTreeMap::new();
        let mut current: Option<&str> = None;
        for date in dates {
            let claim = self.active_claim(&due.id, date);
            if let Some(claim) = claim.filter(|c| c.valid_from == date) {
                balances.clear();
                for p in &claim.parties {
                    balances.insert(
                        p.entity_id.clone(),
                        p.economic_share
                            .as_ref()
                            .map(parse_fraction)
                            .transpose()?
                            .map(|w| &outstanding * w),
                    );
                }
                current = Some(&claim.id);
            }
            let cash = self
                .rentals
                .allocations
                .iter()
                .filter(|a| {
                    a.due_id == due.id && a.component == component && a.effective_on == date
                })
                .try_fold(BigRational::zero(), |sum, a| {
                    Ok(sum + parse_decimal(&a.amount)?)
                })?;
            outstanding -= &cash;
            if let Some(claim) = claim.filter(|c| Some(c.id.as_str()) == current) {
                for p in &claim.parties {
                    if let (Some(Some(value)), Some(share)) =
                        (balances.get_mut(&p.entity_id), p.economic_share.as_ref())
                    {
                        *value -= &cash * parse_fraction(share)?;
                    }
                }
            }
            for m in self.treasury.deposits.iter().filter(|m| {
                m.kind == "applied_to_due"
                    && m.due_id.as_deref() == Some(&due.id)
                    && m.component == component
                    && m.effective_on == date
            }) {
                let amount = -parse_decimal(&m.amount)?;
                outstanding -= &amount;
                if let Some(Some(value)) = balances.get_mut(&m.recipient_entity_id) {
                    *value -= amount;
                }
            }
        }
        if self
            .active_claim(&due.id, at)
            .is_none_or(|c| Some(c.id.as_str()) != current)
        {
            balances.clear();
        }
        Ok(balances)
    }
    fn claim_adjustment(&self, due: &str, entity: &str, date: &str) -> Result<BigRational> {
        self.finance
            .rent_claim_adjustments
            .iter()
            .filter(|a| {
                a.due_id == due && a.creditor_entity_id == entity && a.effective_on.as_str() <= date
            })
            .try_fold(BigRational::zero(), |sum, a| {
                Ok(sum + parse_decimal(&a.amount)?)
            })
    }
    fn rate(&self, currency: &str, date: &str) -> Result<(Option<BigRational>, FxResult)> {
        let conversion = crate::fx::convert_currency_unchecked(
            self.registry,
            &FxQuery {
                amount: "1".into(),
                from_currency: currency.into(),
                to_currency: "EUR".into(),
                date: date.into(),
                knowledge_sequence: Some(self.registry.knowledge_sequence),
            },
        )?;
        Ok((
            conversion.exact_rate.as_ref().map(exact).transpose()?,
            conversion,
        ))
    }
}

/// Financial source validation supplements, rather than replaces, each module's
/// invariants. In particular an application of a deposit spends its own holder's
/// claim, never another co-owner's right by an implicit proportional transfer.
pub fn validate_finance(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    finance: &FinanceSnapshot,
) -> Result<()> {
    validate_properties(registry, properties)?;
    validate_rentals(registry, properties, rentals)?;
    validate_debts(registry, debts)?;
    validate_treasury(registry, properties, rentals, treasury)?;
    validate_treasury_debt_links(registry, treasury, debts)?;
    if [
        properties.knowledge_sequence,
        rentals.knowledge_sequence,
        treasury.knowledge_sequence,
        debts.knowledge_sequence,
        finance.knowledge_sequence,
    ]
    .iter()
    .any(|k| *k != registry.knowledge_sequence)
    {
        return Err(invalid(
            "Les sources financières doivent partager la même borne de connaissance.",
        ));
    }
    let view = View {
        registry,
        properties,
        rentals,
        treasury,
        debts,
        finance,
    };
    let mut ids = HashSet::new();
    let mut revisions = HashSet::new();
    for (id, rev) in finance
        .coverages
        .iter()
        .map(|v| (&v.id, &v.revision_id))
        .chain(finance.rent_claims.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(
            finance
                .rent_claim_adjustments
                .iter()
                .map(|v| (&v.id, &v.revision_id)),
        )
    {
        identifier(id)?;
        identifier(rev)?;
        if !ids.insert(id) || !revisions.insert(rev) {
            return Err(invalid("Une identité financière apparaît plusieurs fois."));
        }
    }
    let mut coverage: Vec<_> = finance.coverages.iter().collect();
    coverage.sort_by_key(|c| &c.valid_from);
    for c in &coverage {
        validate_interval(&c.valid_from, c.valid_to.as_deref())?;
        text(&c.notes, 10000, false)?;
    }
    if coverage.windows(2).any(|p| {
        p[0].valid_to
            .as_deref()
            .is_none_or(|end| p[1].valid_from.as_str() < end)
    }) {
        return Err(invalid("Deux déclarations de couverture se chevauchent."));
    }
    let mut periods: BTreeMap<&str, Vec<&RentClaim>> = BTreeMap::new();
    let mut boundaries = BTreeSet::new();
    for claim in &finance.rent_claims {
        let due = rentals
            .dues
            .iter()
            .find(|d| d.id == claim.due_id)
            .ok_or_else(|| invalid("Le droit locatif concerne une échéance inconnue."))?;
        validate_interval(&claim.valid_from, claim.valid_to.as_deref())?;
        if claim.valid_from < due.period_start {
            return Err(invalid(
                "Le droit de créance ne peut précéder la période de l’échéance.",
            ));
        }
        text(&claim.notes, 10000, false)?;
        let mut parties = HashSet::new();
        let mut sum = BigRational::zero();
        if claim.parties.is_empty() {
            return Err(invalid("Identifiez les créanciers de l’échéance."));
        }
        for p in &claim.parties {
            if !registry.entities.iter().any(|e| e.id == p.entity_id)
                || !parties.insert(&p.entity_id)
            {
                return Err(invalid("Un créancier locatif est inconnu ou dupliqué."));
            }
            if let Some(share) = &p.economic_share {
                sum += parse_fraction(share)?;
            }
        }
        if sum > BigRational::one() {
            return Err(invalid("Les droits de créance dépassent 100 %."));
        }
        if let Some(debt) = &claim.represented_by_debt_id {
            if !debts
                .contracts
                .iter()
                .any(|d| d.id == *debt && d.currency == due.currency)
            {
                return Err(invalid(
                    "Le contrat représentant la créance est inconnu ou de devise différente.",
                ));
            }
        }
        periods.entry(&claim.due_id).or_default().push(claim);
        boundaries.insert(claim.valid_from.clone());
        if let Some(end) = &claim.valid_to {
            boundaries.insert(end.clone());
        }
    }
    for rows in periods.values_mut() {
        rows.sort_by_key(|c| &c.valid_from);
        if rows.windows(2).any(|p| {
            p[0].valid_to
                .as_deref()
                .is_none_or(|end| p[1].valid_from.as_str() < end)
        }) {
            return Err(invalid("Deux droits sur une même échéance se chevauchent."));
        }
    }
    for a in &finance.rent_claim_adjustments {
        if !rentals.dues.iter().any(|d| d.id == a.due_id)
            || !finance.rent_claims.iter().any(|c| {
                c.due_id == a.due_id
                    && c.parties
                        .iter()
                        .any(|p| p.entity_id == a.creditor_entity_id)
            })
        {
            return Err(invalid(
                "La dépréciation concerne un créancier non déclaré de cette échéance.",
            ));
        }
        validate_date(&a.effective_on)?;
        text(&a.source, 1000, true)?;
        text(&a.notes, 10000, false)?;
        let delta = parse_decimal(&a.amount)?;
        if !((a.kind == "impairment" && delta.is_negative())
            || (a.kind == "reversal" && delta > BigRational::zero()))
        {
            return Err(invalid(
                "Une dépréciation est négative ; une reprise est positive.",
            ));
        }
        boundaries.insert(a.effective_on.clone());
    }
    for a in &rentals.allocations {
        boundaries.insert(a.effective_on.clone());
    }
    for m in &treasury.deposits {
        boundaries.insert(m.effective_on.clone());
    }
    for at in &boundaries {
        for claim in finance
            .rent_claims
            .iter()
            .filter(|c| applies(&c.valid_from, c.valid_to.as_deref(), at))
        {
            let due = rentals
                .dues
                .iter()
                .find(|d| d.id == claim.due_id)
                .expect("validated due");
            for p in &claim.parties {
                let adjustment = view.claim_adjustment(&due.id, &p.entity_id, at)?;
                if adjustment > BigRational::zero() {
                    return Err(invalid("La reprise dépasse les dépréciations antérieures."));
                }
                if let Some(remaining) = view.party_remaining(due, claim, &p.entity_id, at)? {
                    for component in ["rent", "charges"] {
                        if view
                            .party_components(due, component, at)?
                            .get(&p.entity_id)
                            .and_then(|v| v.as_ref())
                            .is_some_and(|v| v.is_negative())
                        {
                            return Err(invalid(
                                "Une compensation dépasse la composante détenue par ce créancier.",
                            ));
                        }
                    }
                    if remaining.is_negative() || remaining + adjustment < BigRational::zero() {
                        return Err(invalid("Le règlement ou la dépréciation dépasse le reliquat de ce créancier. Enregistrez une reprise ou régularisation explicite."));
                    }
                } else if !adjustment.is_zero() {
                    return Err(invalid(
                        "Une dépréciation requiert des droits de créance connus.",
                    ));
                }
            }
            for old in finance.rent_claim_adjustments.iter().filter(|a| {
                a.due_id == due.id
                    && !claim
                        .parties
                        .iter()
                        .any(|p| p.entity_id == a.creditor_entity_id)
            }) {
                if !view
                    .claim_adjustment(&due.id, &old.creditor_entity_id, at)?
                    .is_zero()
                {
                    return Err(invalid("Régularisez la dépréciation de l’ancien créancier avant de transférer la créance."));
                }
            }
        }
    }
    for movement in treasury
        .deposits
        .iter()
        .filter(|m| m.kind == "applied_to_due")
    {
        let due = rentals
            .dues
            .iter()
            .find(|d| Some(d.id.as_str()) == movement.due_id.as_deref())
            .ok_or_else(|| invalid("Échéance de compensation inconnue."))?;
        let claim = view
            .active_claim(&due.id, &movement.effective_on)
            .ok_or_else(|| {
                invalid("Déclarez les créanciers de l’échéance avant d’appliquer le dépôt.")
            })?;
        if claim.represented_by_debt_id.is_some() {
            return Err(invalid("La créance est représentée par un financement ; régularisez sa source unique avant compensation."));
        }
        if !claim
            .parties
            .iter()
            .any(|p| p.entity_id == movement.recipient_entity_id && p.economic_share.is_some())
        {
            return Err(invalid(
                "Le détenteur du dépôt doit posséder un droit explicite sur l’échéance compensée.",
            ));
        }
    }
    Ok(())
}

#[derive(Clone, Default)]
struct Exposure {
    visible: BigRational,
    hidden: BTreeMap<String, BigRational>,
    unknown: bool,
    sources: BTreeSet<String>,
}
impl Exposure {
    fn total(&self) -> BigRational {
        self.hidden
            .values()
            .fold(self.visible.clone(), |s, v| s + v)
    }
}
fn combine<'a>(
    rows: impl Iterator<Item = (&'a str, &'a str, &'a str, &'a Option<Fraction>)>,
    weights: &BTreeMap<String, Exposure>,
    scope: bool,
) -> Result<Exposure> {
    let mut result = Exposure::default();
    let mut sum = BigRational::zero();
    for (owner, id, rev, share) in rows {
        let parent = weights.get(owner).cloned().unwrap_or_default();
        match share {
            Some(f) => {
                let f = parse_fraction(f)?;
                sum += &f;
                result.visible += &parent.visible * &f;
                for (k, w) in &parent.hidden {
                    *result
                        .hidden
                        .entry(k.clone())
                        .or_insert_with(BigRational::zero) += w * &f;
                }
                result.unknown |= parent.unknown && !f.is_zero();
                if !f.is_zero() && (!parent.total().is_zero() || parent.unknown) {
                    result.sources.extend(parent.sources);
                    result.sources.extend([id.into(), rev.into()]);
                }
            }
            None if !parent.total().is_zero() || parent.unknown => {
                result.unknown = true;
                result.sources.extend(parent.sources);
                result.sources.extend([id.into(), rev.into()]);
            }
            None => {}
        }
        if result.total().numer().bits() + result.total().denom().bits() > 131072 {
            return Err(AppError::new(
                "capacity",
                "Le graphe financier dépasse le budget de précision.",
            ));
        }
    }
    result.unknown |= scope && sum < BigRational::one();
    Ok(result)
}
fn exposures(
    registry: &RegistrySnapshot,
    q: &FinancialReportRequest,
) -> Result<BTreeMap<String, Exposure>> {
    let mut result = BTreeMap::new();
    for entity in registry.entities.iter().filter(|e| e.kind == "person") {
        let selected = if q.context == "family" {
            registry.memberships.iter().any(|m| {
                m.person_id == entity.id && applies(&m.valid_from, m.valid_to.as_deref(), &q.date)
            })
        } else {
            q.person_id.as_deref() == Some(&entity.id)
        };
        let mut weight = Exposure {
            visible: if selected {
                BigRational::one()
            } else {
                BigRational::zero()
            },
            ..Default::default()
        };
        if selected {
            weight
                .sources
                .extend([entity.id.clone(), entity.revision_id.clone()]);
            for m in registry.memberships.iter().filter(|m| {
                q.context == "family"
                    && m.person_id == entity.id
                    && applies(&m.valid_from, m.valid_to.as_deref(), &q.date)
            }) {
                weight.sources.extend([m.id.clone(), m.revision_id.clone()]);
            }
        }
        result.insert(entity.id.clone(), weight);
    }
    let scope = result.values().any(|e| !e.total().is_zero());
    for company in crate::validation::company_order(registry, &q.date)? {
        let mut weight = combine(
            registry
                .participations
                .iter()
                .filter(|p| {
                    p.company_id == company
                        && applies(&p.valid_from, p.valid_to.as_deref(), &q.date)
                })
                .map(|p| {
                    (
                        p.owner_entity_id.as_str(),
                        p.id.as_str(),
                        p.revision_id.as_str(),
                        &p.economic_share,
                    )
                }),
            &result,
            scope,
        )?;
        let policy = registry.company_policies.iter().find(|p| {
            p.company_id == company && applies(&p.valid_from, p.valid_to.as_deref(), &q.date)
        });
        if !policy.is_some_and(|p| p.mode == "detailed") {
            *weight
                .hidden
                .entry(company.clone())
                .or_insert_with(BigRational::zero) += weight.visible.clone();
            weight.visible = BigRational::zero();
            if policy.is_none() && !weight.total().is_zero() {
                weight.unknown = true;
            }
        }
        if let Some(p) = policy {
            weight.sources.extend([p.id.clone(), p.revision_id.clone()]);
        }
        result.insert(company, weight);
    }
    Ok(result)
}
fn asset_exposure(
    registry: &RegistrySnapshot,
    weights: &BTreeMap<String, Exposure>,
    asset: &str,
    date: &str,
) -> Result<Exposure> {
    combine(
        registry
            .ownerships
            .iter()
            .filter(|o| o.asset_id == asset && applies(&o.valid_from, o.valid_to.as_deref(), date))
            .map(|o| {
                (
                    o.owner_entity_id.as_str(),
                    o.id.as_str(),
                    o.revision_id.as_str(),
                    &o.economic_share,
                )
            }),
        weights,
        weights.values().any(|e| !e.total().is_zero()),
    )
}

pub fn financial_asset_weight(
    registry: &RegistrySnapshot,
    asset: &str,
    context: &str,
    person_id: Option<&str>,
    date: &str,
) -> Result<EconomicWeight> {
    validate_registry(registry)?;
    validate_date(date)?;
    if context != "family" && context != "person" {
        return Err(invalid("Périmètre de stock financier invalide."));
    }
    if !registry.assets.iter().any(|a| a.id == asset) {
        return Err(invalid("Actif financier inconnu."));
    }
    if (context == "family" && person_id.is_some())
        || (context == "person"
            && !registry
                .entities
                .iter()
                .any(|e| e.kind == "person" && Some(e.id.as_str()) == person_id))
    {
        return Err(invalid("Membre financier inconnu."));
    }
    let q = FinancialReportRequest {
        context: context.into(),
        person_id: person_id.map(str::to_owned),
        date: date.into(),
        knowledge_sequence: Some(registry.knowledge_sequence),
    };
    let e = asset_exposure(registry, &exposures(registry, &q)?, asset, date)?;
    Ok(EconomicWeight {
        weight: fraction_of(&e.visible),
        unknown: e.unknown,
        source_ids: e.sources.into_iter().collect(),
    })
}
pub fn financial_entity_weight(
    registry: &RegistrySnapshot,
    entity: &str,
    context: &str,
    person_id: Option<&str>,
    date: &str,
) -> Result<EconomicWeight> {
    validate_registry(registry)?;
    validate_date(date)?;
    if !registry.entities.iter().any(|e| e.id == entity) {
        return Err(invalid("Entité financière inconnue."));
    }
    if !((context == "family" && person_id.is_none())
        || (context == "person"
            && registry
                .entities
                .iter()
                .any(|e| e.kind == "person" && Some(e.id.as_str()) == person_id)))
    {
        return Err(invalid("Périmètre financier invalide."));
    }
    let q = FinancialReportRequest {
        context: context.into(),
        person_id: person_id.map(str::to_owned),
        date: date.into(),
        knowledge_sequence: Some(registry.knowledge_sequence),
    };
    let e = exposures(registry, &q)?
        .get(entity)
        .cloned()
        .unwrap_or_default();
    Ok(EconomicWeight {
        weight: fraction_of(&e.visible),
        unknown: e.unknown,
        source_ids: e.sources.into_iter().collect(),
    })
}

struct Pending {
    id: String,
    label: String,
    kind: String,
    entity: Option<String>,
    country: Option<String>,
    currency: String,
    native: Option<BigRational>,
    weight: Option<BigRational>,
    sources: Vec<String>,
    unknown: bool,
    liability: bool,
    contingent: bool,
}
impl Pending {
    fn new(
        id: &str,
        label: &str,
        kind: &str,
        currency: &str,
        native: Option<BigRational>,
        weight: Option<BigRational>,
        liability: bool,
    ) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: kind.into(),
            entity: None,
            country: None,
            currency: currency.into(),
            native,
            weight,
            sources: Vec::new(),
            unknown: false,
            liability,
            contingent: false,
        }
    }
}
struct Book {
    positions: Vec<FinancialPosition>,
    eliminations: Vec<FinancialElimination>,
    warnings: Vec<FinancialWarning>,
    sources: BTreeSet<String>,
    assets: BigRational,
    liabilities: BigRational,
    assets_known: bool,
    liabilities_known: bool,
}
impl Book {
    fn new() -> Self {
        Self {
            positions: Vec::new(),
            eliminations: Vec::new(),
            warnings: Vec::new(),
            sources: BTreeSet::new(),
            assets: BigRational::zero(),
            liabilities: BigRational::zero(),
            assets_known: true,
            liabilities_known: true,
        }
    }
    fn warn(&mut self, code: &str, id: &str, message: &str) {
        if !self
            .warnings
            .iter()
            .any(|w| w.code == code && w.object_id == id && w.message == message)
        {
            self.warnings.push(FinancialWarning {
                code: code.into(),
                object_id: id.into(),
                message: message.into(),
            });
        }
    }
    fn incomplete(&mut self, liability: bool) {
        if liability {
            self.liabilities_known = false
        } else {
            self.assets_known = false
        }
    }
    fn add(&mut self, view: &View<'_>, date: &str, p: Pending) -> Result<()> {
        if p.weight.as_ref().is_some_and(Zero::is_zero) && !p.unknown {
            return Ok(());
        }
        let mut sources: BTreeSet<String> = p.sources.into_iter().collect();
        sources.insert(p.id.clone());
        let (rate, conversion) = view.rate(&p.currency, date)?;
        for w in &conversion.warnings {
            self.warn(&w.code, &p.id, &w.message);
        }
        for leg in &conversion.legs {
            sources.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
        }
        let eur = match (&p.native, &p.weight, rate) {
            (Some(n), Some(w), Some(rate)) => Some(n * w * rate),
            _ => None,
        };
        if (p.unknown || eur.is_none()) && !p.contingent {
            self.incomplete(p.liability);
        }
        if p.unknown {
            self.warn("unknown_financial_allocation",&p.id,"Une fraction ou une couverture de cette position reste inconnue. Seule la partie calculable figure dans les sous-totaux connus.");
        }
        if !p.contingent {
            if let Some(value) = &eur {
                if p.liability {
                    self.liabilities += value
                } else {
                    self.assets += value
                }
            }
        }
        self.sources.extend(sources.iter().cloned());
        self.positions.push(FinancialPosition {
            object_id: p.id,
            label: p.label,
            kind: p.kind,
            entity_id: p.entity,
            country: p.country,
            currency: p.currency.clone(),
            amount_original: p
                .native
                .as_ref()
                .map(|n| format_currency(n, &p.currency))
                .transpose()?,
            exact_original: p.native.as_ref().map(fraction_of),
            weight: p.weight.as_ref().map(fraction_of),
            amount_eur: eur.as_ref().map(format_eur),
            exact_eur: eur.as_ref().map(fraction_of),
            source_ids: sources.into_iter().collect(),
            conversion: Some(conversion),
        });
        Ok(())
    }
    #[allow(clippy::too_many_arguments)] // The two nominal and carrying faces must be passed together.
    fn eliminate(
        &mut self,
        view: &View<'_>,
        date: &str,
        id: &str,
        label: &str,
        currency: &str,
        c: BigRational,
        d: BigRational,
        nominal_c: BigRational,
        nominal_d: BigRational,
        sources: Vec<String>,
    ) -> Result<()> {
        if nominal_c.is_zero() || nominal_d.is_zero() {
            return Ok(());
        }
        // Match the same nominal fraction, then only the carrying amount shared
        // by both faces. An impairment must leave its unmatched liability visible.
        let paired = nominal_c.clone().min(nominal_d.clone());
        let credit_book = &paired * &c / &nominal_c;
        let debt_book = &paired * &d / &nominal_d;
        let e = credit_book.min(debt_book);
        if e.is_zero() {
            return Ok(());
        }
        let (rate, conversion) = view.rate(currency, date)?;
        let Some(rate) = rate else {
            self.assets_known = false;
            self.liabilities_known = false;
            return Ok(());
        };
        let e = &e * &rate;
        self.assets -= &e;
        self.liabilities -= &e;
        let mut ids: BTreeSet<String> = sources.into_iter().collect();
        for leg in conversion.legs {
            ids.extend([leg.rate_id, leg.revision_id]);
        }
        self.sources.extend(ids.iter().cloned());
        self.eliminations.push(FinancialElimination {
            object_id: id.into(),
            label: label.into(),
            creditor_before_eur: format_eur(&(&c * &rate)),
            debtor_before_eur: format_eur(&(&d * &rate)),
            eliminated_eur: format_eur(&e),
            creditor_after_eur: format_eur(&(c * &rate - &e)),
            debtor_after_eur: format_eur(&(d * &rate - &e)),
            exact_eliminated: fraction_of(&e),
            source_ids: ids.into_iter().collect(),
        });
        Ok(())
    }
}

fn add_debts(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    book: &mut Book,
) -> Result<()> {
    for contract in view
        .debts
        .contracts
        .iter()
        .filter(|c| c.tracking_start <= q.date)
    {
        let report = debt_report(
            view.registry,
            view.debts,
            &DebtReportRequest {
                debt_id: contract.id.clone(),
                date: q.date.clone(),
                knowledge_sequence: q.knowledge_sequence,
            },
        )?;
        let mut c_nominal = BigRational::zero();
        let mut d_nominal = BigRational::zero();
        let mut c_value = BigRational::zero();
        let mut d_value = BigRational::zero();
        let mut c_known = true;
        let mut d_known = true;
        let mut c_shares = BigRational::zero();
        let mut d_shares = BigRational::zero();
        if report.terms.is_none() {
            book.assets_known = false;
            book.liabilities_known = false;
            book.warn(
                "missing_debt_terms",
                &contract.id,
                "Les parties du financement ne sont pas connues à cette date.",
            );
        }
        for party in &report.parties {
            let creditor = party.party.side == "creditor";
            let exposure = debt_weight(
                view,
                weights,
                &party.party.entity_id,
                &contract.id,
                &party.party.side,
                &q.date,
            );
            let nominal = party.exact_nominal.as_ref().map(exact).transpose()?;
            let carrying = if creditor {
                party.exact_carrying_value.as_ref().map(exact).transpose()?
            } else {
                nominal.clone()
            };
            let side_known = nominal.is_some() && carrying.is_some() && !exposure.unknown;
            if creditor {
                c_known &= side_known;
            } else {
                d_known &= side_known;
            }
            if let Some(share) = &party.party.economic_share {
                if creditor {
                    c_shares += parse_fraction(share)?;
                } else {
                    d_shares += parse_fraction(share)?;
                }
            }
            let n = nominal
                .as_ref()
                .map(|n| n * &exposure.visible)
                .unwrap_or_else(BigRational::zero);
            let value = carrying
                .as_ref()
                .map(|n| n * &exposure.visible)
                .unwrap_or_else(BigRational::zero);
            if creditor {
                c_nominal += n;
                c_value += &value;
            } else {
                d_nominal += n;
                d_value += &value;
            }
            let mut p = Pending::new(
                &contract.id,
                &contract.name,
                if creditor {
                    "loan_receivable"
                } else {
                    "loan_debt"
                },
                &contract.currency,
                carrying,
                Some(exposure.visible.clone()),
                !creditor,
            );
            p.entity = Some(party.party.entity_id.clone());
            p.sources = report.source_ids.clone();
            p.sources.extend(exposure.sources.iter().cloned());
            p.unknown = exposure.unknown;
            if !creditor
                && !exposure.visible.is_zero()
                && view
                    .registry
                    .entities
                    .iter()
                    .any(|e| e.id == party.party.entity_id && e.kind == "company")
                && !view.registry.company_policies.iter().any(|c| {
                    c.company_id == party.party.entity_id
                        && c.liability_qualified
                        && applies(&c.valid_from, c.valid_to.as_deref(), &q.date)
                })
            {
                p.unknown = true;
                book.warn("unqualified_company_liability",&contract.id,"La dette de société est une exposition économique analytique ; la responsabilité des associés doit être qualifiée avant un net complet, sans dette personnelle présumée.");
            }
            if exposure.unknown && !exposure.hidden.is_empty() {
                book.warn("opaque_debt_coverage_unknown",&contract.id,"Une face du prêt traverse une valeur globale sans déclaration de couverture précise. Aucune élimination réciproque n’est supposée.");
            }
            book.add(view, &q.date, p)?;
        }
        if c_shares != BigRational::one() {
            c_known = false;
            book.assets_known = false;
            book.warn(
                "incomplete_creditor_allocation",
                &contract.id,
                "Les parts économiques des créanciers ne totalisent pas 100 % connus.",
            );
        }
        if d_shares != BigRational::one() {
            d_known = false;
            book.liabilities_known = false;
            book.warn(
                "incomplete_debtor_allocation",
                &contract.id,
                "Les parts économiques des débiteurs ne totalisent pas 100 % connus.",
            );
        }
        if c_known && d_known {
            book.eliminate(
                view,
                &q.date,
                &contract.id,
                &contract.name,
                &contract.currency,
                c_value,
                d_value,
                c_nominal,
                d_nominal,
                report.source_ids.clone(),
            )?;
        }
        for warning in report.warnings {
            book.warn(&warning.code, &warning.object_id, &warning.message);
        }
        for guarantee in report.guarantees {
            let e = weights
                .get(&guarantee.guarantor_entity_id)
                .cloned()
                .unwrap_or_default();
            let mut p = Pending::new(
                &guarantee.id,
                &format!("Garantie — {}", contract.name),
                "guarantee",
                &contract.currency,
                guarantee
                    .limit_amount
                    .as_ref()
                    .map(|v| parse_decimal(v))
                    .transpose()?,
                Some(e.total()),
                false,
            );
            p.entity = Some(guarantee.guarantor_entity_id);
            p.sources = vec![guarantee.id, guarantee.revision_id, contract.id.clone()];
            p.contingent = true;
            p.unknown = e.unknown;
            book.add(view, &q.date, p)?;
            book.warn("contingent_guarantee",&contract.id,"Les garanties sont visibles hors passifs certains et ne sont pas soustraites du net.");
        }
    }
    Ok(())
}

fn add_rent_claims(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    book: &mut Book,
) -> Result<()> {
    for due in view.rentals.dues.iter().filter(|d| d.due_on <= q.date) {
        let total =
            view.remaining(due, "rent", &q.date)? + view.remaining(due, "charges", &q.date)?;
        if total.is_zero() {
            continue;
        }
        let lease = view
            .rentals
            .leases
            .iter()
            .find(|l| l.id == due.lease_id)
            .expect("validated lease");
        let tenant = weights
            .get(&lease.tenant_entity_id)
            .cloned()
            .unwrap_or_default();
        let Some(claim) = view.active_claim(&due.id, &q.date) else {
            book.assets_known = false;
            book.warn("missing_rent_creditors",&due.id,"Les créanciers de l’échéance ne sont pas déclarés à cette date. Le propriétaire actuel n’est pas utilisé comme créancier historique.");
            let mut p = Pending::new(
                &due.id,
                "Loyer exigible — créanciers inconnus",
                "rent_receivable",
                &due.currency,
                None,
                None,
                false,
            );
            p.sources = vec![due.id.clone(), due.revision_id.clone()];
            book.add(view, &q.date, p)?;
            let mut p = Pending::new(
                &due.id,
                "Loyer exigible — locataire",
                "rent_debt",
                &due.currency,
                Some(total),
                Some(tenant.visible.clone()),
                true,
            );
            p.entity = Some(lease.tenant_entity_id.clone());
            p.sources = tenant.sources.into_iter().collect();
            p.unknown = tenant.unknown;
            book.add(view, &q.date, p)?;
            continue;
        };
        if let Some(debt_id) = &claim.represented_by_debt_id {
            let opening = debt_balance(view.registry, view.debts, debt_id, &claim.valid_from)?;
            let at_transfer = view.remaining(due, "rent", &claim.valid_from)?
                + view.remaining(due, "charges", &claim.valid_from)?;
            if opening
                .exact_principal
                .as_ref()
                .map(exact)
                .transpose()?
                .as_ref()
                != Some(&at_transfer)
            {
                book.assets_known = false;
                book.liabilities_known = false;
                book.warn("rent_loan_representation_mismatch",&due.id,"Le contrat représentant la créance ne concorde pas avec le reliquat à la date du transfert. La créance n’a pas été ajoutée une seconde fois.");
            }
            book.sources.extend([
                claim.id.clone(),
                claim.revision_id.clone(),
                debt_id.clone(),
                due.id.clone(),
                due.revision_id.clone(),
            ]);
            continue;
        }
        let mut nominal = BigRational::zero();
        let mut carrying = BigRational::zero();
        let mut shares = BigRational::zero();
        let mut known = true;
        let mut sources = vec![
            due.id.clone(),
            due.revision_id.clone(),
            claim.id.clone(),
            claim.revision_id.clone(),
            lease.id.clone(),
            lease.revision_id.clone(),
        ];
        for a in view
            .rentals
            .allocations
            .iter()
            .filter(|a| a.due_id == due.id && a.effective_on <= q.date)
        {
            sources.extend([a.id.clone(), a.revision_id.clone()]);
        }
        for m in view
            .treasury
            .deposits
            .iter()
            .filter(|m| m.due_id.as_deref() == Some(&due.id) && m.effective_on <= q.date)
        {
            sources.extend([m.id.clone(), m.revision_id.clone()]);
        }
        for party in &claim.parties {
            let exposure = weights.get(&party.entity_id).cloned().unwrap_or_default();
            let balance = view.party_remaining(due, claim, &party.entity_id, &q.date)?;
            let adjustment = view.claim_adjustment(&due.id, &party.entity_id, &q.date)?;
            if let Some(s) = &party.economic_share {
                shares += parse_fraction(s)?;
            }
            known &= balance.is_some() && !exposure.unknown;
            let carrying_party = balance.as_ref().map(|b| b + &adjustment);
            nominal += balance
                .as_ref()
                .map(|b| b * &exposure.visible)
                .unwrap_or_else(BigRational::zero);
            carrying += carrying_party
                .as_ref()
                .map(|b| b * &exposure.visible)
                .unwrap_or_else(BigRational::zero);
            let mut p = Pending::new(
                &due.id,
                "Créance locative",
                "rent_receivable",
                &due.currency,
                carrying_party,
                Some(exposure.visible.clone()),
                false,
            );
            p.entity = Some(party.entity_id.clone());
            p.sources = sources.clone();
            p.sources.extend(exposure.sources.iter().cloned());
            p.unknown = exposure.unknown;
            for a in view.finance.rent_claim_adjustments.iter().filter(|a| {
                a.due_id == due.id
                    && a.creditor_entity_id == party.entity_id
                    && a.effective_on <= q.date
            }) {
                p.sources.extend([a.id.clone(), a.revision_id.clone()]);
            }
            book.add(view, &q.date, p)?;
        }
        if shares != BigRational::one() {
            known = false;
            book.assets_known = false;
            book.warn(
                "incomplete_rent_creditors",
                &due.id,
                "Les droits de créance locative ne totalisent pas 100 % connus.",
            );
        }
        let liability = &total * &tenant.visible;
        let mut p = Pending::new(
            &due.id,
            "Dette locative du locataire",
            "rent_debt",
            &due.currency,
            Some(total.clone()),
            Some(tenant.visible.clone()),
            true,
        );
        p.entity = Some(lease.tenant_entity_id.clone());
        p.sources = sources.clone();
        p.sources.extend(tenant.sources.iter().cloned());
        p.unknown = tenant.unknown;
        book.add(view, &q.date, p)?;
        if known && !tenant.unknown {
            book.eliminate(
                view,
                &q.date,
                &due.id,
                "Élimination locative interne",
                &due.currency,
                carrying,
                liability.clone(),
                nominal,
                liability,
                sources,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)] // A restitution is one source with two contractual faces.
fn add_reciprocal(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    book: &mut Book,
    id: &str,
    label: &str,
    kind: &str,
    lease_id: &str,
    holder: &str,
    currency: &str,
    amount: BigRational,
    sources: Vec<String>,
) -> Result<()> {
    if amount.is_zero() {
        return Ok(());
    }
    let lease = view
        .rentals
        .leases
        .iter()
        .find(|l| l.id == lease_id)
        .ok_or_else(|| invalid("Bail de restitution inconnu."))?;
    let creditor = weights
        .get(&lease.tenant_entity_id)
        .cloned()
        .unwrap_or_default();
    let debtor = weights.get(holder).cloned().unwrap_or_default();
    let c = &amount * &creditor.visible;
    let d = &amount * &debtor.visible;
    let mut p = Pending::new(
        id,
        label,
        &format!("{kind}_receivable"),
        currency,
        Some(amount.clone()),
        Some(creditor.visible.clone()),
        false,
    );
    p.entity = Some(lease.tenant_entity_id.clone());
    p.sources = sources.clone();
    p.sources.extend(creditor.sources.iter().cloned());
    p.unknown = creditor.unknown;
    book.add(view, &q.date, p)?;
    let mut p = Pending::new(
        id,
        label,
        &format!("{kind}_liability"),
        currency,
        Some(amount),
        Some(debtor.visible.clone()),
        true,
    );
    p.entity = Some(holder.into());
    p.sources = sources.clone();
    p.sources.extend(debtor.sources.iter().cloned());
    p.unknown = debtor.unknown;
    book.add(view, &q.date, p)?;
    if !creditor.unknown && !debtor.unknown {
        book.eliminate(
            view,
            &q.date,
            id,
            label,
            currency,
            c.clone(),
            d.clone(),
            c,
            d,
            sources,
        )?;
    }
    Ok(())
}

fn add_advances_and_deposits(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    book: &mut Book,
) -> Result<()> {
    let mut deposits: BTreeMap<(&str, &str, &str), Vec<&DepositMovement>> = BTreeMap::new();
    for m in view
        .treasury
        .deposits
        .iter()
        .filter(|m| m.effective_on <= q.date)
    {
        deposits
            .entry((&m.lease_id, &m.recipient_entity_id, &m.currency))
            .or_default()
            .push(m);
    }
    for ((lease, holder, currency), movements) in deposits {
        let amount = deposit_obligation_at(view.treasury, lease, holder, &q.date)?;
        let sources = movements
            .iter()
            .flat_map(|m| [m.id.clone(), m.revision_id.clone()])
            .collect();
        add_reciprocal(
            view,
            q,
            weights,
            book,
            &format!("deposit-{lease}-{holder}"),
            "Dépôt de garantie à restituer",
            "deposit",
            lease,
            holder,
            currency,
            amount,
            sources,
        )?;
    }
    let mut advances: BTreeMap<(&str, &str, &str), (BigRational, Vec<String>)> = BTreeMap::new();
    for p in view
        .rentals
        .payments
        .iter()
        .filter(|p| p.effective_on <= q.date)
    {
        let mut unearned = parse_decimal(&p.amount)?;
        for a in view
            .rentals
            .allocations
            .iter()
            .filter(|a| a.payment_id == p.id && a.effective_on <= q.date)
        {
            if view
                .rentals
                .dues
                .iter()
                .any(|d| d.id == a.due_id && d.due_on <= q.date)
            {
                unearned -= parse_decimal(&a.amount)?;
            }
        }
        let entry = advances
            .entry((&p.lease_id, &p.recipient_entity_id, &p.currency))
            .or_insert_with(|| (BigRational::zero(), Vec::new()));
        entry.0 += unearned;
        entry.1.extend([p.id.clone(), p.revision_id.clone()]);
    }
    for ((lease, holder, currency), (amount, sources)) in advances {
        if amount.is_negative() {
            return Err(invalid(
                "Une avance locataire ne peut pas être négative à cette date.",
            ));
        }
        add_reciprocal(
            view,
            q,
            weights,
            book,
            &format!("advance-{lease}-{holder}"),
            "Avance locative non acquise",
            "advance",
            lease,
            holder,
            currency,
            amount,
            sources,
        )?;
    }
    Ok(())
}

fn source_linked(
    view: &View<'_>,
    kind: &str,
    id: &str,
    revision: &str,
    amount: &BigRational,
) -> Result<bool> {
    let linked = view
        .treasury
        .cash_links
        .iter()
        .filter(|l| l.source_kind == kind && l.source_id == id && l.source_revision_id == revision)
        .try_fold(BigRational::zero(), |sum, l| {
            Ok(sum + parse_decimal(&l.amount)?)
        })?;
    Ok(linked == *amount)
}
fn check_rental_sources(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    book: &mut Book,
) -> Result<()> {
    for expense in view
        .rentals
        .expenses
        .iter()
        .filter(|e| e.paid_on.as_deref().is_some_and(|d| d <= q.date.as_str()))
    {
        let exposure = asset_exposure(view.registry, weights, &expense.asset_id, &q.date)?;
        if (!exposure.total().is_zero() || exposure.unknown)
            && !source_linked(
                view,
                "rental_expense",
                &expense.id,
                &expense.revision_id,
                &(-parse_decimal(&expense.amount)?),
            )?
        {
            book.assets_known = false;
            book.warn("unlinked_rental_expense_cash",&expense.id,"Une dépense déclarée payée n’est pas entièrement rapprochée de son débit exact. Le journal n’a pas été diminué par hypothèse.");
        }
    }
    for p in view
        .rentals
        .payments
        .iter()
        .filter(|p| p.effective_on <= q.date)
    {
        let e = weights
            .get(&p.recipient_entity_id)
            .cloned()
            .unwrap_or_default();
        if (!e.total().is_zero() || e.unknown)
            && !source_linked(
                view,
                "rent_payment",
                &p.id,
                &p.revision_id,
                &parse_decimal(&p.amount)?,
            )?
        {
            book.assets_known = false;
            book.warn("unlinked_rent_cash",&p.id,"Un paiement locatif n’est pas entièrement rapproché d’une ligne de trésorerie. Aucune banque n’a été créée par hypothèse.");
        }
    }
    for m in view
        .treasury
        .deposits
        .iter()
        .filter(|m| m.effective_on <= q.date && (m.kind == "received" || m.kind == "returned"))
    {
        let e = weights
            .get(&m.recipient_entity_id)
            .cloned()
            .unwrap_or_default();
        let amount = parse_decimal(&m.amount)?;
        if (!e.total().is_zero() || e.unknown)
            && !source_linked(view, "deposit_movement", &m.id, &m.revision_id, &amount)?
        {
            book.assets_known = false;
            book.warn("unlinked_deposit_cash",&m.id,"Un dépôt reçu ou restitué n’est pas rapproché de son mouvement réel de trésorerie.");
        }
    }
    for lease in view
        .rentals
        .leases
        .iter()
        .filter(|l| l.valid_from <= q.date)
    {
        let Some(unit) = view.properties.units.iter().find(|u| u.id == lease.unit_id) else {
            continue;
        };
        let e = asset_exposure(view.registry, weights, &unit.property_asset_id, &q.date)?;
        if e.total().is_zero() && !e.unknown {
            continue;
        }
        let tomorrow = time::Date::from_calendar_date(
            q.date[..4]
                .parse()
                .map_err(|_| invalid("Année invalide."))?,
            time::Month::try_from(
                q.date[5..7]
                    .parse::<u8>()
                    .map_err(|_| invalid("Mois invalide."))?,
            )
            .map_err(|_| invalid("Mois invalide."))?,
            q.date[8..].parse().map_err(|_| invalid("Jour invalide."))?,
        )
        .map_err(|_| invalid("Date invalide."))?
        .next_day()
        .ok_or_else(|| invalid("Date hors limite."))?;
        let observed_end = format!(
            "{:04}-{:02}-{:02}",
            tomorrow.year(),
            u8::from(tomorrow.month()),
            tomorrow.day()
        );
        let to = lease
            .valid_to
            .as_deref()
            .unwrap_or(&observed_end)
            .min(&observed_end);
        if to <= lease.valid_from.as_str() {
            continue;
        }
        let report = rental_report(
            view.registry,
            view.properties,
            view.rentals,
            &RentalReportRequest {
                context: q.context.clone(),
                person_id: q.person_id.clone(),
                asset_id: Some(unit.property_asset_id.clone()),
                country: None,
                from_date: lease.valid_from.clone(),
                to_date: to.into(),
                date: q.date.clone(),
                knowledge_sequence: q.knowledge_sequence,
            },
        )?;
        for w in report
            .warnings
            .into_iter()
            .filter(|w| w.code == "missing_rent_dues" || w.code == "missing_lease_terms")
        {
            book.assets_known = false;
            book.warn(&w.code, &w.object_id, &w.message);
        }
    }
    Ok(())
}

fn check_debt_cash_sources(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    book: &mut Book,
) -> Result<()> {
    for movement in view.debts.movements.iter().filter(|m| {
        m.effective_on <= q.date && ["drawdown", "principal_repayment"].contains(&m.kind.as_str())
    }) {
        let terms = view.debts.terms.iter().find(|t| {
            t.debt_id == movement.debt_id
                && applies(&t.valid_from, t.valid_to.as_deref(), &movement.effective_on)
        });
        let Some(terms) = terms else {
            book.assets_known = false;
            book.warn(
                "unknown_debt_cash_parties",
                &movement.id,
                "Les parties à la date du flux de financement ne sont pas connues.",
            );
            continue;
        };
        for side in ["creditor", "debtor"] {
            let parties: Vec<_> = terms
                .parties
                .iter()
                .filter(|p| {
                    p.side == side
                        && weights
                            .get(&p.entity_id)
                            .is_some_and(|e| e.unknown || !e.total().is_zero())
                })
                .collect();
            if parties.is_empty() {
                continue;
            }
            let mut fraction = BigRational::zero();
            let mut known = true;
            for party in &parties {
                match &party.economic_share {
                    Some(f) => fraction += parse_fraction(f)?,
                    None => known = false,
                }
            }
            for (component, expected) in [
                (
                    "principal",
                    parse_decimal(&movement.principal_change)?.abs(),
                ),
                ("interest", parse_decimal(&movement.interest_paid)?),
                ("fee", parse_decimal(&movement.fees_paid)?),
            ] {
                if expected.is_zero() || (side == "creditor" && component == "fee") {
                    continue;
                }
                let mut total = BigRational::zero();
                let mut scoped = BigRational::zero();
                for link in view.treasury.cash_links.iter().filter(|l| {
                    l.source_kind == "debt_movement"
                        && l.source_id == movement.id
                        && l.source_revision_id == movement.revision_id
                        && l.side == side
                        && l.component == component
                }) {
                    let amount = parse_decimal(&link.amount)?.abs();
                    total += &amount;
                    let owner = view
                        .treasury
                        .transactions
                        .iter()
                        .find(|t| t.revision_id == link.transaction_revision_id)
                        .and_then(|t| t.entries.iter().find(|e| e.line_id == link.line_id))
                        .and_then(|e| view.treasury.accounts.iter().find(|a| a.id == e.account_id))
                        .map(|a| a.legal_owner_entity_id.as_str());
                    if owner.is_some_and(|id| parties.iter().any(|p| p.entity_id == id)) {
                        scoped += amount;
                    }
                }
                if total != expected && (!known || scoped < expected * &fraction) {
                    book.assets_known = false;
                    book.warn("unlinked_debt_cash",&movement.id,"Un tirage ou remboursement réel n’est pas entièrement rapproché, par côté et composante, des lignes exactes de trésorerie concernant le périmètre. Aucun cash n’a été inventé.");
                }
            }
        }
    }
    Ok(())
}

fn debt_weight(
    view: &View<'_>,
    weights: &BTreeMap<String, Exposure>,
    entity: &str,
    debt: &str,
    side: &str,
    date: &str,
) -> Exposure {
    let mut e = weights.get(entity).cloned().unwrap_or_default();
    for (company, weight) in &e.hidden {
        if weight.is_zero() {
            continue;
        }
        match view.debts.coverages.iter().find(|c| {
            c.debt_id == debt
                && c.company_id == *company
                && c.side == side
                && applies(&c.valid_from, c.valid_to.as_deref(), date)
        }) {
            Some(c) => {
                e.sources.extend([c.id.clone(), c.revision_id.clone()]);
                if !c.included {
                    e.visible += weight;
                }
            }
            None => {
                e.unknown = true;
            }
        }
    }
    e
}

/// The headline totals require both calculable sources and an explicit dated
/// declaration of inventory coverage. Known subtotals remain reviewable when
/// a source is missing; an empty table never asserts that no liability exists.
pub fn financial_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    finance: &FinanceSnapshot,
    q: &FinancialReportRequest,
) -> Result<FinancialReport> {
    financial_report_inner(
        registry, properties, rentals, treasury, debts, finance, None, None, q,
    )
}

/// The same balance includes exact metal stocks and checks actual distributions.
#[allow(clippy::too_many_arguments)] // Independent source snapshots share one knowledge sequence.
pub fn financial_report_with_operations(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    finance: &FinanceSnapshot,
    metals: &MetalSnapshot,
    companies: &CompanySnapshot,
    q: &FinancialReportRequest,
) -> Result<FinancialReport> {
    validate_metals(registry, properties, treasury, metals)?;
    validate_companies(registry, properties, rentals, treasury, debts, companies)?;
    validate_operation_cash_capacity(treasury, metals, companies)?;
    if metals.knowledge_sequence != finance.knowledge_sequence
        || companies.knowledge_sequence != finance.knowledge_sequence
    {
        return Err(invalid(
            "Les opérations et le bilan ne partagent pas les mêmes connaissances.",
        ));
    }
    financial_report_inner(
        registry,
        properties,
        rentals,
        treasury,
        debts,
        finance,
        Some(metals),
        Some(companies),
        q,
    )
}

#[allow(clippy::too_many_arguments)] // Optional sources preserve the phase 6 public contract.
fn financial_report_inner(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    finance: &FinanceSnapshot,
    metals: Option<&MetalSnapshot>,
    companies: Option<&CompanySnapshot>,
    q: &FinancialReportRequest,
) -> Result<FinancialReport> {
    validate_finance(registry, properties, rentals, treasury, debts, finance)?;
    validate_date(&q.date)?;
    if q.knowledge_sequence
        .is_some_and(|k| k != finance.knowledge_sequence)
    {
        return Err(invalid(
            "La borne financière ne correspond pas aux sources.",
        ));
    }
    if !((q.context == "family" && q.person_id.is_none())
        || (q.context == "person"
            && registry
                .entities
                .iter()
                .any(|e| e.kind == "person" && Some(&e.id) == q.person_id.as_ref())))
    {
        return Err(invalid("Le périmètre financier est invalide."));
    }
    let view = View {
        registry,
        properties,
        rentals,
        treasury,
        debts,
        finance,
    };
    let weights = exposures(registry, q)?;
    let mut book = Book::new();
    let coverage = finance
        .coverages
        .iter()
        .find(|c| applies(&c.valid_from, c.valid_to.as_deref(), &q.date));
    let coverage_confirmed =
        coverage.is_some_and(|c| c.assets_complete && c.debts_complete && c.rentals_complete);
    if let Some(c) = coverage {
        book.sources.extend([c.id.clone(), c.revision_id.clone()]);
    }
    if !coverage.is_some_and(|c| c.assets_complete) {
        book.assets_known = false;
        book.warn(
            "assets_coverage_unconfirmed",
            "scope",
            "La couverture des actifs et comptes n’est pas confirmée pour cette date.",
        );
    }
    if !coverage.is_some_and(|c| c.debts_complete) {
        book.liabilities_known = false;
        book.warn("debts_coverage_unconfirmed","scope","L’inventaire des passifs n’est pas confirmé : une table vide ne signifie pas aucune dette.");
    }
    if !coverage.is_some_and(|c| c.rentals_complete) {
        book.assets_known = false;
        book.liabilities_known = false;
        book.warn(
            "rentals_coverage_unconfirmed",
            "scope",
            "La couverture des créances, avances et dépôts locatifs n’est pas confirmée.",
        );
    }
    let account_assets: HashSet<_> = treasury
        .accounts
        .iter()
        .map(|a| a.asset_id.as_str())
        .collect();
    let metal_assets: HashSet<_> = metals
        .into_iter()
        .flat_map(|s| s.lots.iter())
        .map(|l| l.asset_id.as_str())
        .collect();
    let sold: HashSet<_> = properties
        .events
        .iter()
        .filter(|e| e.kind == "disposal" && e.effective_on <= q.date)
        .map(|e| e.asset_id.as_str())
        .collect();
    let mut stock = registry.clone();
    stock.valuations.retain(|v| {
        v.target_kind != "asset"
            || !(account_assets.contains(v.target_id.as_str())
                || metal_assets.contains(v.target_id.as_str()))
    });
    let base = consolidate(
        &stock,
        &ConsolidationRequest {
            context: q.context.clone(),
            person_id: q.person_id.clone(),
            date: q.date.clone(),
            knowledge_sequence: q.knowledge_sequence,
        },
    )?;
    for w in base.warnings {
        if w.code == "liabilities_not_inventoried"
            || (w.code == "missing_valuation"
                && (account_assets.contains(w.object_id.as_str())
                    || metal_assets.contains(w.object_id.as_str())))
            || (w.code == "unknown_economic_rights" && sold.contains(w.object_id.as_str()))
        {
            continue;
        }
        if ![
            "undated_profile",
            "stale_exchange_rate",
            "inconsistent_exchange_rates",
        ]
        .contains(&w.code.as_str())
        {
            book.assets_known = false;
        }
        book.warn(&w.code, &w.object_id, &w.message);
    }
    for c in base.contributions {
        let value = exact(&c.exact_eur)?;
        book.assets += &value;
        book.sources.extend(c.source_ids.iter().cloned());
        book.positions.push(FinancialPosition {
            object_id: c.object_id,
            label: c.label,
            kind: c.kind,
            entity_id: None,
            country: c.country,
            currency: c.currency.clone(),
            amount_original: Some(c.amount_original.clone()),
            exact_original: Some(fraction_of(&parse_decimal(&c.amount_original)?)),
            weight: Some(c.weight),
            amount_eur: Some(format_eur(&value)),
            exact_eur: Some(fraction_of(&value)),
            source_ids: c.source_ids,
            conversion: c.conversion,
        });
    }
    for account in &treasury.accounts {
        let exposure = asset_exposure(registry, &weights, &account.asset_id, &q.date)?;
        let report = treasury_report(
            registry,
            properties,
            rentals,
            treasury,
            &TreasuryReportRequest {
                account_id: account.id.clone(),
                date: q.date.clone(),
                knowledge_sequence: q.knowledge_sequence,
            },
        )?;
        let balance = report.exact_balance.as_ref().map(exact).transpose()?;
        let negative = balance.as_ref().is_some_and(Signed::is_negative);
        let mut p = Pending::new(
            &account.id,
            &registry
                .assets
                .iter()
                .find(|a| a.id == account.asset_id)
                .map(|a| a.name.clone())
                .unwrap_or_else(|| account.id.clone()),
            if negative { "overdraft" } else { "treasury" },
            &account.currency,
            balance.map(|v| v.abs()),
            Some(exposure.visible.clone()),
            negative,
        );
        p.sources = report.source_ids;
        p.sources.extend(exposure.sources.iter().cloned());
        p.unknown = exposure.unknown;
        p.entity = Some(account.legal_owner_entity_id.clone());
        p.country = Some(account.country.clone());
        if negative {
            let titles: Vec<_> = registry
                .ownerships
                .iter()
                .filter(|o| {
                    o.asset_id == account.asset_id
                        && applies(&o.valid_from, o.valid_to.as_deref(), &q.date)
                })
                .collect();
            if titles.len() != 1
                || titles[0].owner_entity_id != account.legal_owner_entity_id
                || titles[0]
                    .economic_share
                    .as_ref()
                    .is_none_or(|s| s.num != "1" || s.den != "1")
            {
                p.unknown = true;
                p.weight = None;
                book.warn("overdraft_allocation_unknown",&account.id,"Le découvert d’un compte à droits multiples nécessite une allocation économique de dette explicite ; il n’est pas divisé automatiquement selon les droits sur les liquidités.");
            }
        }
        if let Some(last) = report
            .reconciliations
            .iter()
            .max_by_key(|r| &r.effective_on)
        {
            if last
                .exact_difference
                .as_ref()
                .map(exact)
                .transpose()?
                .is_some_and(|v| !v.is_zero())
                && (!exposure.visible.is_zero() || exposure.unknown)
            {
                p.unknown = true;
                book.warn("unresolved_reconciliation",&account.id,"Le dernier relevé présente un écart non résolu. Le solde calculé n’a pas été ajusté implicitement.");
            }
        }
        for w in report.warnings {
            book.warn(&w.code, &w.object_id, &w.message);
        }
        book.add(&view, &q.date, p)?;
    }
    add_debts(&view, q, &weights, &mut book)?;
    add_rent_claims(&view, q, &weights, &mut book)?;
    add_advances_and_deposits(&view, q, &weights, &mut book)?;
    if let Some(metals) = metals {
        add_metal_stocks(&view, q, &weights, metals, &mut book)?;
    }
    if let Some(companies) = companies {
        check_distribution_sources(&view, q, &weights, companies, &mut book)?;
    }
    check_rental_sources(&view, q, &weights, &mut book)?;
    check_debt_cash_sources(&view, q, &weights, &mut book)?;
    let net = &book.assets - &book.liabilities;
    let complete = book.assets_known && book.liabilities_known;
    Ok(FinancialReport {
        context: q.context.clone(),
        person_id: q.person_id.clone(),
        date: q.date.clone(),
        knowledge_sequence: finance.knowledge_sequence,
        status: if complete { "complete" } else { "partial" }.into(),
        coverage_confirmed,
        assets_eur: book.assets_known.then(|| format_eur(&book.assets)),
        liabilities_eur: book
            .liabilities_known
            .then(|| format_eur(&book.liabilities)),
        net_eur: complete.then(|| format_eur(&net)),
        exact_assets: book.assets_known.then(|| fraction_of(&book.assets)),
        exact_liabilities: book
            .liabilities_known
            .then(|| fraction_of(&book.liabilities)),
        exact_net: complete.then(|| fraction_of(&net)),
        known_assets_eur: format_eur(&book.assets),
        known_liabilities_eur: format_eur(&book.liabilities),
        known_net_eur: format_eur(&net),
        positions: book.positions,
        eliminations: book.eliminations,
        source_ids: book.sources.into_iter().collect(),
        warnings: book.warnings,
    })
}

fn add_metal_stocks(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    metals: &MetalSnapshot,
    book: &mut Book,
) -> Result<()> {
    for lot in &metals.lots {
        let exposure = asset_exposure(view.registry, weights, &lot.asset_id, &q.date)?;
        if exposure.visible.is_zero() && !exposure.unknown {
            continue;
        }
        let report = metal_report(
            view.registry,
            view.properties,
            view.treasury,
            metals,
            &MetalReportRequest {
                lot_id: lot.id.clone(),
                date: q.date.clone(),
                knowledge_sequence: q.knowledge_sequence,
            },
        )?;
        let native = if lot.valuation_basis == "intrinsic" {
            report.exact_intrinsic_original.as_ref()
        } else {
            report.exact_resale_original.as_ref()
        };
        let mut p = Pending::new(
            &lot.id,
            &lot.designation,
            "metal",
            &report.currency,
            native.map(exact).transpose()?,
            Some(exposure.visible),
            false,
        );
        p.country = Some(report.country);
        p.unknown = exposure.unknown;
        p.sources = report.source_ids;
        p.sources.extend(exposure.sources);
        for w in report.warnings {
            book.warn(&w.code, &w.object_id, &w.message);
        }
        if native.is_none() {
            book.warn("metal_stock_valuation_missing", &lot.id, "Le stock de métal ne dispose pas de toutes les sources requises par sa méthode de valorisation explicite.");
        }
        book.add(view, &q.date, p)?;
    }
    for movement in metals
        .movements
        .iter()
        .filter(|m| ["purchase", "sale"].contains(&m.kind.as_str()) && m.effective_on <= q.date)
    {
        let exposure = weights
            .get(&movement.cash_entity_id)
            .cloned()
            .unwrap_or_default();
        if (!exposure.visible.is_zero() || exposure.unknown)
            && !metal_cash_coverage(metals, movement)?
        {
            book.incomplete(false);
            book.warn("unlinked_metal_trade_cash", &movement.id, "Un achat ou une vente de métal n’est pas entièrement rapproché de son mouvement exact de trésorerie. Aucun paiement ni produit de cession n’a été ajouté par hypothèse.");
        }
        let lot = metals
            .lots
            .iter()
            .find(|l| l.id == movement.lot_id)
            .ok_or_else(|| invalid("Lot d’opération inconnu."))?;
        let stock_exposure = asset_exposure(
            view.registry,
            weights,
            &lot.asset_id,
            &movement.effective_on,
        )?;
        let sign = if movement.kind == "purchase" {
            BigRational::one()
        } else {
            -BigRational::one()
        };
        let mut hidden_changes: BTreeMap<String, BigRational> = BTreeMap::new();
        for (company, share) in stock_exposure.hidden {
            *hidden_changes.entry(company).or_default() += &share * &sign;
        }
        for (company, share) in exposure.hidden {
            *hidden_changes.entry(company).or_default() -= &share * &sign;
        }
        require_equity_after_operation(
            view,
            q,
            &hidden_changes,
            &movement.effective_on,
            "opaque_equity_before_metal_operation",
            book,
        );
    }
    check_metal_ownership_boundaries(view, q, weights, metals, book)?;
    Ok(())
}

fn check_distribution_sources(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    companies: &CompanySnapshot,
    book: &mut Book,
) -> Result<()> {
    for distribution in companies
        .distributions
        .iter()
        .filter(|d| d.kind == "paid" && d.effective_on <= q.date)
    {
        let relevant = [
            &distribution.company_id,
            &distribution.beneficiary_entity_id,
        ]
        .iter()
        .any(|id| {
            weights
                .get(*id)
                .is_some_and(|e| !e.total().is_zero() || e.unknown)
        });
        if relevant {
            book.sources
                .extend([distribution.id.clone(), distribution.revision_id.clone()]);
            for link in &distribution.cash_links {
                book.sources
                    .extend([link.transaction_revision_id.clone(), link.line_id.clone()]);
            }
        }
        let (payer, beneficiary) = distribution_cash_coverage(companies, distribution)?;
        for (entity, covered) in [
            (&distribution.company_id, payer),
            (&distribution.beneficiary_entity_id, beneficiary),
        ] {
            let exposure = weights.get(entity).cloned().unwrap_or_default();
            if (!exposure.visible.is_zero() || exposure.unknown) && !covered {
                book.incomplete(false);
                book.warn("unlinked_dividend_cash", &distribution.id, "Une distribution déclarée payée n’est pas rapprochée sur chaque face visible du périmètre. Un résultat ou un indicateur ne remplace pas la trésorerie réellement versée.");
            }
        }
        // A cash flow crossing an opaque boundary changes the equity it
        // represents. An earlier estimate cannot certify the post-payment net.
        // Matching movements inside the same opaque perimeter cancel first.
        let mut hidden_changes: BTreeMap<String, BigRational> = BTreeMap::new();
        let amount = parse_decimal(&distribution.amount)?;
        for (entity, sign) in [
            (&distribution.company_id, -BigRational::one()),
            (&distribution.beneficiary_entity_id, BigRational::one()),
        ] {
            if let Some(exposure) = weights.get(entity) {
                for (company, share) in &exposure.hidden {
                    *hidden_changes.entry(company.clone()).or_default() += &amount * share * &sign;
                }
            }
        }
        require_equity_after_operation(
            view,
            q,
            &hidden_changes,
            &distribution.effective_on,
            "opaque_equity_before_distribution",
            book,
        );
    }
    Ok(())
}

fn require_equity_after_operation(
    view: &View<'_>,
    q: &FinancialReportRequest,
    changes: &BTreeMap<String, BigRational>,
    effective_on: &str,
    warning_code: &str,
    book: &mut Book,
) {
    for (company, change) in changes {
        if change.is_zero() {
            continue;
        }
        let latest = view
            .registry
            .valuations
            .iter()
            .filter(|v| {
                v.target_kind == "company" && &v.target_id == company && v.effective_on <= q.date
            })
            .max_by(|a, b| a.effective_on.cmp(&b.effective_on));
        if let Some(value) = latest {
            book.sources
                .extend([value.id.clone(), value.revision_id.clone()]);
        }
        if latest.is_none_or(|v| v.effective_on.as_str() < effective_on) {
            book.incomplete(false);
            book.warn(warning_code, company, "Une opération réelle traverse le périmètre d’une société globale dont la valeur connue précède l’opération. Une estimation tenant compte de cette opération est nécessaire ; aucune correction de valeur des parts n’a été supposée.");
        }
    }
}

fn check_metal_ownership_boundaries(
    view: &View<'_>,
    q: &FinancialReportRequest,
    weights: &BTreeMap<String, Exposure>,
    metals: &MetalSnapshot,
    book: &mut Book,
) -> Result<()> {
    let scope = weights.values().any(|e| !e.total().is_zero());
    for lot in &metals.lots {
        let dates: BTreeSet<_> = view
            .registry
            .ownerships
            .iter()
            .filter(|o| o.asset_id == lot.asset_id)
            .flat_map(|o| std::iter::once(o.valid_from.as_str()).chain(o.valid_to.as_deref()))
            .filter(|date| *date > lot.tracking_start.as_str() && *date <= q.date.as_str())
            .collect();
        for date in dates {
            if metal_quantity_at(metals, &lot.id, date)?.is_some_and(|n| n.is_zero()) {
                continue;
            }
            // The left limit selects rights that ended exactly on this date.
            let before = combine(
                view.registry
                    .ownerships
                    .iter()
                    .filter(|o| {
                        o.asset_id == lot.asset_id
                            && o.valid_from.as_str() < date
                            && o.valid_to.as_deref().is_none_or(|end| end >= date)
                    })
                    .map(|o| {
                        (
                            o.owner_entity_id.as_str(),
                            o.id.as_str(),
                            o.revision_id.as_str(),
                            &o.economic_share,
                        )
                    }),
                weights,
                scope,
            )?;
            let after = asset_exposure(view.registry, weights, &lot.asset_id, date)?;
            let mut changes: BTreeMap<String, BigRational> = BTreeMap::new();
            for (company, share) in before.hidden {
                *changes.entry(company).or_default() -= share;
            }
            for (company, share) in after.hidden {
                *changes.entry(company).or_default() += share;
            }
            require_equity_after_operation(
                view,
                q,
                &changes,
                date,
                "opaque_equity_before_metal_transfer",
                book,
            );
        }
    }
    Ok(())
}
