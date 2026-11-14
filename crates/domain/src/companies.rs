//! Company management observations and explicit distributions, separate from stocks.
use crate::*;
use serde::{Deserialize, Serialize};

macro_rules! company_record {
    ($row:ident,$input:ident,{$($field:ident:$ty:ty),*$(,)?}) => {
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row {pub id:String,pub revision_id:String,$(pub $field:$ty,)*}
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input {pub id:Option<String>,pub expected_revision_id:Option<String>,pub reason:String,$(pub $field:$ty,)*}
    }
}

company_record!(CompanyFinancialPeriod,SaveCompanyFinancialPeriodInput,{
    company_id:String,period_start:String,period_end:String,frequency:String,currency:String,
    revenue:Option<String>,operating_expenses:Option<String>,operating_result:Option<String>,
    net_result:Option<String>,indicative_cash:Option<String>,indicative_debt:Option<String>,
    important_receivables:Option<String>,dividends_declared:Option<String>,
    indicative_global_value:Option<String>,notes:String,
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompanyDistributionCashLink {
    pub side: String,
    pub transaction_revision_id: String,
    pub line_id: String,
    pub amount: String,
}

company_record!(CompanyDistribution,SaveCompanyDistributionInput,{
    company_id:String,beneficiary_entity_id:String,kind:String,effective_on:String,
    amount:String,currency:String,related_declaration_id:Option<String>,
    unlinked_reason:String,notes:String,cash_links:Vec<CompanyDistributionCashLink>,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanySnapshot {
    pub financial_periods: Vec<CompanyFinancialPeriod>,
    pub distributions: Vec<CompanyDistribution>,
    pub knowledge_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostedCompanyDistributionCashLink {
    pub side: String,
    pub line_id: String,
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostCompanyDistributionInput {
    pub expected_sequence: u64,
    pub distribution: SaveCompanyDistributionInput,
    pub transaction: Option<SaveTreasuryTransactionInput>,
    pub existing_transaction_revision_id: Option<String>,
    pub links: Vec<PostedCompanyDistributionCashLink>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompanyReportRequest {
    pub company_id: String,
    pub date: String,
    pub frequency: Option<String>,
    pub knowledge_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyTrend {
    pub metric: String,
    pub previous_period_id: String,
    pub difference_original: String,
    pub exact_difference: Fraction,
    pub percentage: Option<String>,
    pub exact_percentage: Option<Fraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyPeriodAnalysis {
    pub period: CompanyFinancialPeriod,
    pub calculated_operating_result: Option<String>,
    pub operating_result_difference: Option<String>,
    pub trends: Vec<CompanyTrend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyAssetHolding {
    pub asset: Asset,
    pub ownership: Ownership,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyAccountBalance {
    pub account: TreasuryAccount,
    pub report: TreasuryReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyDebtPosition {
    pub debt_id: String,
    pub name: String,
    pub side: String,
    pub currency: String,
    pub economic_share: Option<Fraction>,
    pub amount_original: Option<String>,
    pub exact_amount: Option<Fraction>,
    pub amount_eur: Option<String>,
    pub source_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyDistributionStatus {
    pub distribution: CompanyDistribution,
    pub payer_cash_complete: bool,
    pub beneficiary_cash_complete: bool,
    pub paid_against_declaration: Option<String>,
    pub remaining_declared: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompanyReport {
    pub company_id: String,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub company: Entity,
    pub policy: Option<CompanyPolicy>,
    pub owners: Vec<Participation>,
    pub subsidiaries: Vec<Participation>,
    pub assets: Vec<CompanyAssetHolding>,
    pub accounts: Vec<CompanyAccountBalance>,
    pub debt_positions: Vec<CompanyDebtPosition>,
    pub financial_periods: Vec<CompanyPeriodAnalysis>,
    pub distributions: Vec<CompanyDistributionStatus>,
    pub source_ids: Vec<String>,
    pub warnings: Vec<CompanyWarning>,
}

use crate::exact::{currency_minor_units, format_currency, format_eur, fraction_of, parse_decimal};
use crate::fx::{convert_currency_unchecked, converted_rational};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn text(value: &str, maximum: usize, required: bool) -> Result<()> {
    if value.chars().count() > maximum || required && value.trim().is_empty() {
        return Err(invalid(
            "Un texte de pilotage de société est absent ou trop long.",
        ));
    }
    Ok(())
}
fn identifier(value: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > 128
        || !value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid(
            "Un identifiant de pilotage de société est invalide.",
        ));
    }
    Ok(())
}
fn company_exists(registry: &RegistrySnapshot, id: &str) -> bool {
    registry
        .entities
        .iter()
        .any(|e| e.id == id && e.kind == "company")
}
fn period_months(frequency: &str) -> Result<u32> {
    match frequency {
        "monthly" => Ok(1),
        "quarterly" => Ok(3),
        "annual" => Ok(12),
        _ => Err(invalid(
            "Fréquence de période inconnue : mensuelle, trimestrielle ou annuelle attendue.",
        )),
    }
}
fn observation_end(date: &str) -> String {
    let civil = time::Date::from_calendar_date(
        date[..4].parse().expect("validated year"),
        time::Month::try_from(date[5..7].parse::<u8>().expect("validated month"))
            .expect("validated month"),
        date[8..].parse().expect("validated day"),
    )
    .expect("validated date");
    civil
        .next_day()
        .map(|next| {
            format!(
                "{:04}-{:02}-{:02}",
                next.year(),
                u8::from(next.month()),
                next.day()
            )
        })
        .unwrap_or_else(|| date.into())
}
fn valid_period(period: &CompanyFinancialPeriod) -> Result<()> {
    validate_interval(&period.period_start, Some(&period.period_end))?;
    let months = period_months(&period.frequency)?;
    let y1 = period.period_start[..4]
        .parse::<u32>()
        .expect("validated year");
    let m1 = period.period_start[5..7]
        .parse::<u32>()
        .expect("validated month");
    let y2 = period.period_end[..4]
        .parse::<u32>()
        .expect("validated year");
    let m2 = period.period_end[5..7]
        .parse::<u32>()
        .expect("validated month");
    if !period.period_start.ends_with("-01")
        || !period.period_end.ends_with("-01")
        || y2 * 12 + m2 != y1 * 12 + m1 + months
    {
        return Err(invalid("Une période couvre exactement un, trois ou douze mois civils, du premier jour à la fin exclusive."));
    }
    currency_minor_units(&period.currency)?;
    for value in [
        &period.revenue,
        &period.operating_expenses,
        &period.indicative_debt,
        &period.important_receivables,
        &period.dividends_declared,
    ]
    .into_iter()
    .flatten()
    {
        if parse_decimal(value)?.is_negative() {
            return Err(invalid("Chiffre d’affaires, charges, dettes, créances et dividendes indicatifs doivent être positifs ou nuls."));
        }
    }
    for value in [
        &period.operating_result,
        &period.net_result,
        &period.indicative_cash,
        &period.indicative_global_value,
    ]
    .into_iter()
    .flatten()
    {
        parse_decimal(value)?;
    }
    text(&period.notes, 10000, false)
}

/// Coverage of each actual face. Declarations never constitute a payment.
pub fn distribution_cash_coverage(
    _companies: &CompanySnapshot,
    distribution: &CompanyDistribution,
) -> Result<(bool, bool)> {
    if distribution.kind != "paid" {
        return Ok((false, false));
    }
    let amount = parse_decimal(&distribution.amount)?;
    let sum = |side: &str| -> Result<BigRational> {
        distribution
            .cash_links
            .iter()
            .filter(|l| l.side == side)
            .try_fold(BigRational::zero(), |total, l| {
                Ok(total + parse_decimal(&l.amount)?)
            })
    };
    Ok((sum("payer")? == -&amount, sum("beneficiary")? == amount))
}

pub fn validate_companies(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    companies: &CompanySnapshot,
) -> Result<()> {
    validate_treasury(registry, properties, rentals, treasury)?;
    validate_debts(registry, debts)?;
    validate_treasury_debt_links(registry, treasury, debts)?;
    if companies.knowledge_sequence != registry.knowledge_sequence
        || debts.knowledge_sequence != registry.knowledge_sequence
    {
        return Err(invalid(
            "Sociétés et sources doivent partager la même borne de connaissance.",
        ));
    }
    let mut ids = HashSet::new();
    let mut revisions = HashSet::new();
    for (id, revision) in companies
        .financial_periods
        .iter()
        .map(|p| (&p.id, &p.revision_id))
        .chain(
            companies
                .distributions
                .iter()
                .map(|d| (&d.id, &d.revision_id)),
        )
    {
        identifier(id)?;
        identifier(revision)?;
        if !ids.insert(id.as_str()) || !revisions.insert(revision.as_str()) {
            return Err(invalid(
                "Une identité ou révision de société est dupliquée.",
            ));
        }
    }
    for period in &companies.financial_periods {
        if !company_exists(registry, &period.company_id) {
            return Err(invalid("La société de la période est inconnue."));
        }
        valid_period(period)?;
    }
    for (index, a) in companies.financial_periods.iter().enumerate() {
        for b in &companies.financial_periods[index + 1..] {
            if a.company_id == b.company_id
                && a.frequency == b.frequency
                && a.period_start < b.period_end
                && b.period_start < a.period_end
            {
                return Err(invalid("Deux observations de même fréquence se chevauchent pour cette société ; corrigez la période existante."));
            }
        }
    }
    let transactions: HashMap<_, _> = treasury
        .transactions
        .iter()
        .map(|t| (t.revision_id.as_str(), t))
        .collect();
    let accounts: HashMap<_, _> = treasury
        .accounts
        .iter()
        .map(|a| (a.id.as_str(), a))
        .collect();
    let mut capacities: BTreeMap<(&str, &str), BigRational> = BTreeMap::new();
    for link in &treasury.cash_links {
        *capacities
            .entry((&link.transaction_revision_id, &link.line_id))
            .or_default() += parse_decimal(&link.amount)?.abs();
    }
    for distribution in &companies.distributions {
        if !company_exists(registry, &distribution.company_id)
            || !registry
                .entities
                .iter()
                .any(|e| e.id == distribution.beneficiary_entity_id)
            || distribution.company_id == distribution.beneficiary_entity_id
        {
            return Err(invalid(
                "La société ou le bénéficiaire de distribution est inconnu ou identique.",
            ));
        }
        if !["declared", "paid"].contains(&distribution.kind.as_str()) {
            return Err(invalid(
                "Une distribution est déclarée ou réellement payée.",
            ));
        }
        validate_date(&distribution.effective_on)?;
        currency_minor_units(&distribution.currency)?;
        let amount = parse_decimal(&distribution.amount)?;
        if amount <= BigRational::zero() {
            return Err(invalid(
                "Le montant d’une distribution doit être strictement positif.",
            ));
        }
        text(&distribution.notes, 10000, false)?;
        text(&distribution.unlinked_reason, 4000, false)?;
        if distribution.kind == "declared"
            && (!distribution.cash_links.is_empty()
                || distribution.related_declaration_id.is_some())
        {
            return Err(invalid(
                "Une déclaration ne porte ni flux de banque ni autre déclaration.",
            ));
        }
        if let Some(parent_id) = &distribution.related_declaration_id {
            let parent = companies
                .distributions
                .iter()
                .find(|d| &d.id == parent_id)
                .ok_or_else(|| invalid("La déclaration source est inconnue."))?;
            if parent.kind != "declared"
                || parent.company_id != distribution.company_id
                || parent.beneficiary_entity_id != distribution.beneficiary_entity_id
                || parent.currency != distribution.currency
                || parent.effective_on > distribution.effective_on
            {
                return Err(invalid("Le paiement ne correspond pas à sa déclaration antérieure, société, bénéficiaire et devise."));
            }
        }
        let mut seen = HashSet::new();
        let mut totals: BTreeMap<&str, BigRational> = BTreeMap::new();
        for link in &distribution.cash_links {
            if !["payer", "beneficiary"].contains(&link.side.as_str())
                || !seen.insert((&link.side, &link.transaction_revision_id, &link.line_id))
            {
                return Err(invalid(
                    "Une face ou un lien de distribution est inconnu ou dupliqué.",
                ));
            }
            let transaction = transactions
                .get(link.transaction_revision_id.as_str())
                .ok_or_else(|| {
                    invalid("La révision du journal de distribution n’est pas courante.")
                })?;
            let line = transaction
                .entries
                .iter()
                .find(|e| e.line_id == link.line_id)
                .ok_or_else(|| invalid("La ligne du journal de distribution n’existe pas."))?;
            let account = accounts
                .get(line.account_id.as_str())
                .ok_or_else(|| invalid("Le compte du dividende est inconnu."))?;
            let expected_owner = if link.side == "payer" {
                &distribution.company_id
            } else {
                &distribution.beneficiary_entity_id
            };
            let linked = parse_decimal(&link.amount)?;
            let actual = parse_decimal(&line.amount)?;
            if transaction.kind == "opening"
                || transaction.effective_on != distribution.effective_on
                || account.currency != distribution.currency
                || &account.legal_owner_entity_id != expected_owner
                || linked.is_zero()
                || linked.is_positive() != actual.is_positive()
                || (link.side == "payer" && linked.is_positive())
                || (link.side == "beneficiary" && linked.is_negative())
            {
                return Err(invalid(
                    "Face, titulaire, date, devise ou signe du flux de dividende incompatible.",
                ));
            }
            *totals.entry(&link.side).or_default() += linked.abs();
            *capacities
                .entry((&link.transaction_revision_id, &link.line_id))
                .or_default() += linked.abs();
        }
        if totals.values().any(|v| v > &amount) {
            return Err(invalid(
                "Les flux rapprochés dépassent une face de la distribution.",
            ));
        }
        let (payer, beneficiary) = distribution_cash_coverage(companies, distribution)?;
        if distribution.kind == "paid"
            && (!payer || !beneficiary)
            && distribution.unlinked_reason.trim().is_empty()
        {
            return Err(invalid("Une distribution payée incomplètement rapprochée exige un motif historique explicite."));
        }
    }
    for ((revision, line_id), allocated) in capacities {
        // Every treasury CashLink has already been validated above.
        let transaction = transactions
            .get(revision)
            .ok_or_else(|| invalid("Révision de journal absente du contrôle de capacité."))?;
        let line = transaction
            .entries
            .iter()
            .find(|e| e.line_id == line_id)
            .ok_or_else(|| invalid("Ligne absente du contrôle de capacité."))?;
        if allocated > parse_decimal(&line.amount)?.abs() {
            return Err(invalid(
                "Une ligne bancaire ne peut pas payer plusieurs sources au-delà de son montant.",
            ));
        }
    }
    for declaration in companies
        .distributions
        .iter()
        .filter(|d| d.kind == "declared")
    {
        let paid = companies
            .distributions
            .iter()
            .filter(|d| d.related_declaration_id.as_deref() == Some(declaration.id.as_str()))
            .try_fold(BigRational::zero(), |total, d| {
                Ok(total + parse_decimal(&d.amount)?)
            })?;
        if paid > parse_decimal(&declaration.amount)? {
            return Err(invalid(
                "Les paiements dépassent le dividende déclaré ; corrigez ensemble leurs sources.",
            ));
        }
    }
    Ok(())
}

fn exact_fraction(value: &Fraction) -> Result<BigRational> {
    let numerator = value
        .num
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction de rapport invalide."))?;
    let denominator = value
        .den
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction de rapport invalide."))?;
    if denominator <= BigInt::zero() {
        return Err(invalid("Dénominateur de rapport invalide."));
    }
    Ok(BigRational::new(numerator, denominator))
}

fn warning(warnings: &mut Vec<CompanyWarning>, code: &str, id: &str, message: &str) {
    if !warnings
        .iter()
        .any(|w| w.code == code && w.object_id == id && w.message == message)
    {
        warnings.push(CompanyWarning {
            code: code.into(),
            object_id: id.into(),
            message: message.into(),
        });
    }
}

fn trend(
    metric: &str,
    current: Option<&String>,
    previous: Option<&String>,
    period: &CompanyFinancialPeriod,
) -> Result<Option<CompanyTrend>> {
    let (Some(current), Some(previous)) = (current, previous) else {
        return Ok(None);
    };
    let current = parse_decimal(current)?;
    let previous = parse_decimal(previous)?;
    let delta = current - &previous;
    let percent = (!previous.is_zero())
        .then(|| &delta / previous.abs() * BigRational::from_integer(BigInt::from(100)));
    Ok(Some(CompanyTrend {
        metric: metric.into(),
        previous_period_id: period.id.clone(),
        difference_original: format_currency(&delta, &period.currency)?,
        exact_difference: fraction_of(&delta),
        percentage: percent.as_ref().map(format_eur),
        exact_percentage: percent.as_ref().map(fraction_of),
    }))
}

pub fn company_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    companies: &CompanySnapshot,
    request: &CompanyReportRequest,
) -> Result<CompanyReport> {
    validate_companies(registry, properties, rentals, treasury, debts, companies)?;
    validate_date(&request.date)?;
    if request
        .knowledge_sequence
        .is_some_and(|k| k != companies.knowledge_sequence)
    {
        return Err(invalid(
            "La borne de connaissance ne correspond pas au pilotage de société.",
        ));
    }
    if let Some(frequency) = &request.frequency {
        period_months(frequency)?;
    }
    let dated = apply_profile_periods(registry, &request.date)?;
    let company = dated
        .entities
        .iter()
        .find(|e| e.id == request.company_id && e.kind == "company")
        .cloned()
        .ok_or_else(|| invalid("La société demandée est inconnue."))?;
    let mut sources = BTreeSet::from([company.id.clone(), company.revision_id.clone()]);
    for period in registry.profile_periods.iter().filter(|p| {
        p.target_kind == "entity"
            && p.target_id == company.id
            && applies(&p.valid_from, p.valid_to.as_deref(), &request.date)
    }) {
        sources.extend([
            period.id.clone(),
            period.revision_id.clone(),
            period.profile_revision_id.clone(),
        ]);
    }
    let mut warnings = Vec::new();
    let mut incomplete = false;
    if !crate::profiles::profile_is_dated(registry, "entity", &company.id, &request.date) {
        warning(&mut warnings,"undated_company_profile",&company.id,"Le profil descriptif n’est pas daté pour cette observation ; aucun pays historique n’est certifié.");
    }
    let policy = registry
        .company_policies
        .iter()
        .find(|p| {
            p.company_id == company.id
                && applies(&p.valid_from, p.valid_to.as_deref(), &request.date)
        })
        .cloned();
    if let Some(policy) = &policy {
        sources.extend([policy.id.clone(), policy.revision_id.clone()]);
    } else {
        incomplete = true;
        warning(&mut warnings,"missing_company_policy",&company.id,"La méthode de valorisation de la société manque à cette date. Les indicateurs ne la remplacent pas.");
    }
    let owners: Vec<_> = registry
        .participations
        .iter()
        .filter(|p| {
            p.company_id == company.id
                && applies(&p.valid_from, p.valid_to.as_deref(), &request.date)
        })
        .cloned()
        .collect();
    let subsidiaries: Vec<_> = registry
        .participations
        .iter()
        .filter(|p| {
            p.owner_entity_id == company.id
                && applies(&p.valid_from, p.valid_to.as_deref(), &request.date)
        })
        .cloned()
        .collect();
    for p in owners.iter().chain(&subsidiaries) {
        sources.extend([p.id.clone(), p.revision_id.clone()]);
    }
    let mut assets = Vec::new();
    for ownership in registry.ownerships.iter().filter(|o| {
        o.owner_entity_id == company.id
            && applies(&o.valid_from, o.valid_to.as_deref(), &request.date)
    }) {
        if let Some(asset) = dated.assets.iter().find(|a| a.id == ownership.asset_id) {
            sources.extend([
                ownership.id.clone(),
                ownership.revision_id.clone(),
                asset.id.clone(),
                asset.revision_id.clone(),
            ]);
            for period in registry.profile_periods.iter().filter(|p| {
                p.target_kind == "asset"
                    && p.target_id == asset.id
                    && applies(&p.valid_from, p.valid_to.as_deref(), &request.date)
            }) {
                sources.extend([
                    period.id.clone(),
                    period.revision_id.clone(),
                    period.profile_revision_id.clone(),
                ]);
            }
            assets.push(CompanyAssetHolding {
                asset: asset.clone(),
                ownership: ownership.clone(),
            });
        }
    }
    let mut accounts = Vec::new();
    for account in treasury
        .accounts
        .iter()
        .filter(|a| a.legal_owner_entity_id == company.id)
    {
        let report = treasury_report(
            registry,
            properties,
            rentals,
            treasury,
            &TreasuryReportRequest {
                account_id: account.id.clone(),
                date: request.date.clone(),
                knowledge_sequence: request.knowledge_sequence,
            },
        )?;
        if report.balance_original.is_none() || report.balance_eur.is_none() {
            incomplete = true;
        }
        sources.extend(report.source_ids.iter().cloned());
        for w in &report.warnings {
            warning(&mut warnings, &w.code, &w.object_id, &w.message);
        }
        accounts.push(CompanyAccountBalance {
            account: account.clone(),
            report,
        });
    }
    let mut debt_positions = Vec::new();
    for contract in &debts.contracts {
        let relevant = debts.terms.iter().any(|t| {
            t.debt_id == contract.id
                && applies(&t.valid_from, t.valid_to.as_deref(), &request.date)
                && t.parties.iter().any(|p| p.entity_id == company.id)
        });
        if !relevant {
            continue;
        }
        let report = debt_report(
            registry,
            debts,
            &DebtReportRequest {
                debt_id: contract.id.clone(),
                date: request.date.clone(),
                knowledge_sequence: request.knowledge_sequence,
            },
        )?;
        for p in report
            .parties
            .iter()
            .filter(|p| p.party.entity_id == company.id)
        {
            // debt_report has already applied this party's contractual share.
            let allocated = p
                .exact_carrying_value
                .as_ref()
                .map(exact_fraction)
                .transpose()?;
            let conversion = convert_currency_unchecked(
                registry,
                &FxQuery {
                    amount: "1".into(),
                    from_currency: contract.currency.clone(),
                    to_currency: "EUR".into(),
                    date: request.date.clone(),
                    knowledge_sequence: request.knowledge_sequence,
                },
            )?;
            let euro = allocated
                .as_ref()
                .zip(converted_rational(&conversion))
                .map(|(v, rate)| v * rate);
            if allocated.is_none() || euro.is_none() {
                incomplete = true;
            }
            let mut ids: BTreeSet<_> = report.source_ids.iter().cloned().collect();
            for leg in &conversion.legs {
                ids.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
            }
            sources.extend(ids.iter().cloned());
            debt_positions.push(CompanyDebtPosition {
                debt_id: contract.id.clone(),
                name: contract.name.clone(),
                side: p.party.side.clone(),
                currency: contract.currency.clone(),
                economic_share: p.party.economic_share.clone(),
                amount_original: allocated
                    .as_ref()
                    .map(|v| format_currency(v, &contract.currency))
                    .transpose()?,
                exact_amount: allocated.as_ref().map(fraction_of),
                amount_eur: euro.as_ref().map(format_eur),
                source_ids: ids.into_iter().collect(),
            });
        }
        for w in &report.warnings {
            warning(&mut warnings, &w.code, &w.object_id, &w.message);
        }
    }
    let observation_end = observation_end(&request.date);
    let mut periods: Vec<_> = companies
        .financial_periods
        .iter()
        .filter(|p| {
            p.company_id == company.id
                && p.period_end <= observation_end
                && request.frequency.as_ref().is_none_or(|f| &p.frequency == f)
        })
        .cloned()
        .collect();
    periods.sort_by(|a, b| {
        (&a.period_start, &a.frequency, &a.id).cmp(&(&b.period_start, &b.frequency, &b.id))
    });
    let mut financial_periods = Vec::new();
    for period in &periods {
        sources.extend([period.id.clone(), period.revision_id.clone()]);
        let calculated = period
            .revenue
            .as_ref()
            .zip(period.operating_expenses.as_ref())
            .map(|(revenue, expenses)| Ok(parse_decimal(revenue)? - parse_decimal(expenses)?))
            .transpose()?;
        let difference = calculated
            .as_ref()
            .zip(period.operating_result.as_ref())
            .map(|(calculated, stated)| Ok(parse_decimal(stated)? - calculated))
            .transpose()?;
        if difference
            .as_ref()
            .is_some_and(|v: &BigRational| !v.is_zero())
        {
            warning(&mut warnings,"operating_result_discrepancy",&period.id,"Le résultat d’exploitation déclaré diffère de chiffre d’affaires moins charges observées ; aucune source n’a été remplacée.");
        }
        let previous = periods.iter().find(|p| {
            p.frequency == period.frequency
                && p.currency == period.currency
                && p.period_end == period.period_start
        });
        let mut trends = Vec::new();
        if let Some(previous) = previous {
            for candidate in [
                trend(
                    "revenue",
                    period.revenue.as_ref(),
                    previous.revenue.as_ref(),
                    previous,
                )?,
                trend(
                    "operating_result",
                    period.operating_result.as_ref(),
                    previous.operating_result.as_ref(),
                    previous,
                )?,
                trend(
                    "net_result",
                    period.net_result.as_ref(),
                    previous.net_result.as_ref(),
                    previous,
                )?,
            ]
            .into_iter()
            .flatten()
            {
                trends.push(candidate);
            }
        }
        financial_periods.push(CompanyPeriodAnalysis {
            period: period.clone(),
            calculated_operating_result: calculated
                .as_ref()
                .map(|v| format_currency(v, &period.currency))
                .transpose()?,
            operating_result_difference: difference
                .as_ref()
                .map(|v| format_currency(v, &period.currency))
                .transpose()?,
            trends,
        });
    }
    if financial_periods.is_empty() {
        incomplete = true;
        warning(&mut warnings,"company_periods_missing",&company.id,"Aucune période de gestion achevée ne correspond à la fréquence et à la date demandées.");
    }
    if request.frequency.is_none()
        && periods
            .iter()
            .map(|p| &p.frequency)
            .collect::<HashSet<_>>()
            .len()
            > 1
    {
        warning(&mut warnings,"mixed_company_frequencies",&company.id,"Plusieurs fréquences sont affichées séparément ; leurs observations ne sont jamais additionnées.");
    }
    let mut distributions = Vec::new();
    for distribution in companies
        .distributions
        .iter()
        .filter(|d| d.company_id == company.id && d.effective_on <= request.date)
    {
        let (payer, beneficiary) = distribution_cash_coverage(companies, distribution)?;
        if distribution.kind == "paid" && (!payer || !beneficiary) {
            incomplete = true;
            warning(&mut warnings,"unlinked_company_distribution",&distribution.id,"Le paiement de dividende n’est pas entièrement rapproché sur ses deux faces ; aucune trésorerie n’est supposée.");
        }
        if distribution.kind == "paid" && policy.as_ref().is_some_and(|p| p.mode == "global_equity")
        {
            warning(&mut warnings,"opaque_distribution_equity_review",&distribution.id,"La société est valorisée globalement : vérifier une estimation des capitaux propres cohérente après distribution. Le journal ne corrige pas implicitement la valeur globale.");
        }
        sources.extend([distribution.id.clone(), distribution.revision_id.clone()]);
        for link in &distribution.cash_links {
            sources.insert(link.transaction_revision_id.clone());
            sources.insert(link.line_id.clone());
        }
        let paid = if distribution.kind == "declared" {
            Some(
                companies
                    .distributions
                    .iter()
                    .filter(|p| {
                        p.related_declaration_id.as_deref() == Some(distribution.id.as_str())
                            && p.effective_on <= request.date
                    })
                    .try_fold(BigRational::zero(), |total, p| {
                        Ok(total + parse_decimal(&p.amount)?)
                    })?,
            )
        } else {
            None
        };
        let remaining = paid
            .as_ref()
            .map(|paid| Ok(parse_decimal(&distribution.amount)? - paid))
            .transpose()?;
        distributions.push(CompanyDistributionStatus {
            distribution: distribution.clone(),
            payer_cash_complete: payer,
            beneficiary_cash_complete: beneficiary,
            paid_against_declaration: paid
                .as_ref()
                .map(|v| format_currency(v, &distribution.currency))
                .transpose()?,
            remaining_declared: remaining
                .as_ref()
                .map(|v| format_currency(v, &distribution.currency))
                .transpose()?,
        });
    }
    distributions.sort_by(|a, b| {
        (&a.distribution.effective_on, &a.distribution.id)
            .cmp(&(&b.distribution.effective_on, &b.distribution.id))
    });
    warning(&mut warnings,"company_indicators_not_positions",&company.id,"Les observations de pilotage, dont la trésorerie indicative et la valeur globale indicative, n’ajoutent aucun actif ou passif au patrimoine. Cette fiche décrit directement la société, sans attribuer ses montants au membre sélectionné.");
    Ok(CompanyReport {
        company_id: company.id.clone(),
        date: request.date.clone(),
        knowledge_sequence: companies.knowledge_sequence,
        status: if incomplete { "partial" } else { "complete" }.into(),
        company,
        policy,
        owners,
        subsidiaries,
        assets,
        accounts,
        debt_positions,
        financial_periods,
        distributions,
        source_ids: sources.into_iter().collect(),
        warnings,
    })
}
