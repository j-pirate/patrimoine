//! Reports spanning the historical source modules. Cash is counted from its
//! exact ledger links, while a deposit compensation settles a due without a
//! second receipt. Incomplete declared coverage never becomes a zero debt.
use crate::exact::{format_eur, fraction_of, parse_decimal, parse_fraction};
use crate::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::Zero;
use std::collections::BTreeSet;

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn ratio(value: &Fraction) -> Result<BigRational> {
    let numerator = value
        .num
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction de rapport invalide."))?;
    let denominator = value
        .den
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction de rapport invalide."))?;
    if denominator <= BigInt::zero() {
        return Err(invalid("Fraction de rapport invalide."));
    }
    Ok(BigRational::new(numerator, denominator))
}
fn cover<'a>(finance: &'a FinanceSnapshot, date: &str) -> Option<&'a FinanceCoverage> {
    finance
        .coverages
        .iter()
        .find(|c| applies(&c.valid_from, c.valid_to.as_deref(), date))
}
fn non_cash_settlements(treasury: &TreasurySnapshot) -> Result<Vec<NonCashRentSettlement>> {
    treasury
        .deposits
        .iter()
        .filter(|d| d.kind == "applied_to_due")
        .map(|d| {
            Ok(NonCashRentSettlement {
                due_id: d
                    .due_id
                    .clone()
                    .ok_or_else(|| invalid("Échéance du dépôt appliqué inconnue."))?,
                component: d.component.clone(),
                amount: d
                    .amount
                    .strip_prefix('-')
                    .ok_or_else(|| {
                        invalid("Une compensation de dépôt doit réduire son obligation.")
                    })?
                    .to_owned(),
                currency: d.currency.clone(),
                effective_on: d.effective_on.clone(),
                source_ids: vec![d.id.clone(), d.revision_id.clone()],
            })
        })
        .collect()
}

/// Whole-property analytical financing. The loan's debtor shares are never
/// inferred from the ownership of the financed or collateral asset.
pub fn financial_property_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    finance: &FinanceSnapshot,
    q: &PropertyReportRequest,
) -> Result<PropertyReport> {
    validate_finance(registry, properties, rentals, treasury, debts, finance)?;
    let mut out = property_report(registry, properties, q)?;
    out.warnings
        .retain(|w| w.code != "financing_not_inventoried");
    if let Some(coverage) = cover(finance, &q.date) {
        out.source_ids
            .extend([coverage.id.clone(), coverage.revision_id.clone()]);
    }
    if out.disposal.is_some() {
        return Ok(out);
    }
    let Some(coverage) = cover(finance, &q.date).filter(|c| c.debts_complete) else {
        out.warnings.push(PropertyWarning{code:"debt_inventory_not_confirmed".into(),message:"L’inventaire des dettes n’est pas déclaré complet à cette date ; dette affectée et valeur nette restent inconnues.".into()});
        return Ok(out);
    };
    let mut sources: BTreeSet<_> = out.source_ids.iter().cloned().collect();
    sources.extend([coverage.id.clone(), coverage.revision_id.clone()]);
    let mut total = Some(BigRational::zero());
    for link in debts.asset_links.iter().filter(|l| {
        l.asset_id == q.asset_id
            && l.relation_kind == "finances"
            && applies(&l.valid_from, l.valid_to.as_deref(), &q.date)
    }) {
        sources.extend([
            link.id.clone(),
            link.revision_id.clone(),
            link.debt_id.clone(),
        ]);
        let Some(share) = &link.attributed_principal_share else {
            total = None;
            out.warnings.push(PropertyWarning{code:"unknown_financing_allocation".into(),message:"Une affectation analytique de principal à ce bien est inconnue. La propriété du bien ne remplace pas cette information.".into()});
            continue;
        };
        let share = parse_fraction(share)?;
        if share.is_zero() {
            continue;
        }
        let balance = debt_balance(registry, debts, &link.debt_id, &q.date)?;
        sources.extend(balance.source_ids);
        let Some(principal) = balance.exact_principal.as_ref().map(ratio).transpose()? else {
            total = None;
            out.warnings.push(PropertyWarning {
                code: "unknown_financing_principal".into(),
                message: "Le capital d’un financement affecté est incomplet à cette date.".into(),
            });
            continue;
        };
        let conversion = crate::fx::convert_currency_unchecked(
            registry,
            &FxQuery {
                amount: "1".into(),
                from_currency: balance.currency,
                to_currency: "EUR".into(),
                date: q.date.clone(),
                knowledge_sequence: Some(registry.knowledge_sequence),
            },
        )?;
        for warning in &conversion.warnings {
            out.warnings.push(PropertyWarning {
                code: warning.code.clone(),
                message: warning.message.clone(),
            });
        }
        for leg in &conversion.legs {
            sources.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
        }
        if let (Some(total), Some(rate)) = (&mut total, &conversion.exact_rate) {
            *total += principal * share * ratio(rate)?;
        } else {
            total = None;
        }
        out.financing_conversions.push(conversion);
    }
    if let Some(total) = total {
        out.debt_remaining_eur = Some(format_eur(&total));
        out.exact_debt_remaining_eur = Some(fraction_of(&total));
        if let Some(value) = out
            .exact_estimated_value_eur
            .as_ref()
            .map(ratio)
            .transpose()?
        {
            let net = value - total;
            out.net_before_disposal_eur = Some(format_eur(&net));
            out.exact_net_before_disposal_eur = Some(fraction_of(&net));
        }
    }
    out.source_ids = sources.into_iter().collect();
    Ok(out)
}

struct CashFlow<'a> {
    registry: &'a RegistrySnapshot,
    properties: &'a PropertySnapshot,
    rentals: &'a RentalSnapshot,
    treasury: &'a TreasurySnapshot,
    q: &'a RentalReportRequest,
    sources: BTreeSet<String>,
    conversions: Vec<FxResult>,
    warnings: Vec<RentalWarning>,
}
struct CashSource<'a> {
    kind: &'a str,
    id: &'a str,
    revision: &'a str,
    amount: &'a BigRational,
    date: &'a str,
    currency: &'a str,
    component: &'a str,
    side: &'a str,
}
impl CashFlow<'_> {
    fn warning(&mut self, code: &str, id: &str, message: &str) {
        if !self
            .warnings
            .iter()
            .any(|w| w.code == code && w.object_id == id && w.message == message)
        {
            self.warnings.push(RentalWarning {
                code: code.into(),
                object_id: id.into(),
                message: message.into(),
            });
        }
    }
    fn in_period(&self, date: &str) -> bool {
        date >= self.q.from_date.as_str()
            && date < self.q.to_date.as_str()
            && date <= self.q.date.as_str()
    }
    fn selected(&self, asset: &str) -> bool {
        self.q.asset_id.as_deref().is_none_or(|id| id == asset)
            && self.q.country.as_ref().is_none_or(|country| {
                self.registry.profile_periods.iter().any(|p| {
                    p.target_kind == "asset"
                        && p.target_id == asset
                        && applies(&p.valid_from, p.valid_to.as_deref(), &self.q.date)
                }) && asset_at_country(self.registry, asset, &self.q.date).as_ref() == Some(country)
            })
    }
    fn property_of_lease(&self, lease: &str) -> Option<&str> {
        let l = self.rentals.leases.iter().find(|l| l.id == lease)?;
        self.properties
            .units
            .iter()
            .find(|u| u.id == l.unit_id)
            .map(|u| u.property_asset_id.as_str())
    }
    fn fx(
        &mut self,
        n: &BigRational,
        currency: &str,
        date: &str,
        id: &str,
    ) -> Result<Option<BigRational>> {
        if n.is_zero() {
            return Ok(Some(BigRational::zero()));
        }
        let result = crate::fx::convert_currency_unchecked(
            self.registry,
            &FxQuery {
                amount: "1".into(),
                from_currency: currency.into(),
                to_currency: "EUR".into(),
                date: date.into(),
                knowledge_sequence: Some(self.registry.knowledge_sequence),
            },
        )?;
        for w in &result.warnings {
            self.warning(&w.code, id, &w.message);
        }
        for l in &result.legs {
            self.sources
                .extend([l.rate_id.clone(), l.revision_id.clone()]);
        }
        if !self.conversions.iter().any(|old| {
            old.from_currency == result.from_currency
                && old.to_currency == result.to_currency
                && old.date == result.date
                && old
                    .legs
                    .iter()
                    .map(|l| &l.revision_id)
                    .eq(result.legs.iter().map(|l| &l.revision_id))
        }) {
            self.conversions.push(result.clone());
        }
        result
            .exact_rate
            .as_ref()
            .map(|rate| Ok(n * ratio(rate)?))
            .transpose()
    }
    fn account_weight(
        &mut self,
        account: &TreasuryAccount,
        date: &str,
    ) -> Result<Option<BigRational>> {
        self.sources.extend([
            account.id.clone(),
            account.revision_id.clone(),
            account.asset_id.clone(),
        ]);
        if self.q.context == "property" {
            return Ok(Some(BigRational::from_integer(BigInt::from(1))));
        }
        if self.q.context == "person"
            && self
                .registry
                .entities
                .iter()
                .any(|e| e.id == account.legal_owner_entity_id && e.kind == "company")
        {
            return Ok(Some(BigRational::zero()));
        }
        if self.q.context == "company"
            && self.q.person_id.as_deref() != Some(account.legal_owner_entity_id.as_str())
        {
            return Ok(Some(BigRational::zero()));
        }
        let w = if self.q.context == "company" {
            economic_asset_weight(
                self.registry,
                &account.asset_id,
                "company",
                self.q.person_id.as_deref(),
                date,
            )?
        } else {
            financial_asset_weight(
                self.registry,
                &account.asset_id,
                &self.q.context,
                self.q.person_id.as_deref(),
                date,
            )?
        };
        self.sources.extend(w.source_ids);
        if w.unknown {
            self.warning("unknown_cashflow_rights",&account.id,"Les droits du compte à la date du flux sont incomplets ; le cash-flow ne peut pas être totalisé.");
            Ok(None)
        } else {
            Ok(Some(ratio(&w.weight)?))
        }
    }
    fn source_cash(&mut self, source: CashSource<'_>) -> Result<Option<BigRational>> {
        let CashSource {
            kind: source_kind,
            id,
            revision,
            amount,
            date,
            currency,
            component,
            side,
        } = source;
        if amount.is_zero() {
            return Ok(Some(BigRational::zero()));
        }
        let treasury = self.treasury;
        let links: Vec<_> = treasury
            .cash_links
            .iter()
            .filter(|l| {
                l.source_kind == source_kind
                    && l.source_id == id
                    && l.source_revision_id == revision
                    && l.component == component
                    && l.side == side
            })
            .collect();
        let linked = links.iter().try_fold(BigRational::zero(), |sum, l| {
            Ok(sum + parse_decimal(&l.amount)?)
        })?;
        if linked != *amount {
            self.warning("unlinked_cashflow_source",id,"Un flux de la période n’est pas entièrement rapproché avec sa ligne exacte de trésorerie ; aucun montant bancaire n’est fabriqué.");
            return Ok(None);
        }
        let mut total = BigRational::zero();
        for link in links {
            self.sources.extend([
                id.to_owned(),
                revision.to_owned(),
                link.id.clone(),
                link.revision_id.clone(),
                link.transaction_revision_id.clone(),
                link.line_id.clone(),
            ]);
            let tx = treasury
                .transactions
                .iter()
                .find(|t| t.revision_id == link.transaction_revision_id)
                .ok_or_else(|| invalid("Transaction liée inconnue."))?;
            self.sources.insert(tx.id.clone());
            let entry = tx
                .entries
                .iter()
                .find(|e| e.line_id == link.line_id)
                .ok_or_else(|| invalid("Ligne liée inconnue."))?;
            let account = treasury
                .accounts
                .iter()
                .find(|a| a.id == entry.account_id)
                .ok_or_else(|| invalid("Compte lié inconnu."))?;
            if account_balance_at(treasury, &account.id, date)?.is_none() {
                self.warning("cashflow_opening_missing",&account.id,"Une ouverture de compte manque à la date d’un flux ; le cash-flow reste incomplet.");
                return Ok(None);
            }
            let Some(weight) = self.account_weight(account, date)? else {
                return Ok(None);
            };
            total += parse_decimal(&link.amount)? * weight;
        }
        self.fx(&total, currency, date, id)
    }
    fn side_weight(
        &mut self,
        terms: &DebtTerms,
        side: &str,
        date: &str,
    ) -> Result<Option<BigRational>> {
        if self.q.context == "property" {
            return Ok(Some(BigRational::from_integer(BigInt::from(u8::from(
                side == "debtor",
            )))));
        }
        self.sources
            .extend([terms.id.clone(), terms.revision_id.clone()]);
        let mut total = BigRational::zero();
        for party in terms.parties.iter().filter(|p| p.side == side) {
            let w = if self.q.context == "company" {
                let selected = self.q.person_id.as_deref() == Some(party.entity_id.as_str());
                EconomicWeight {
                    weight: fraction_of(&BigRational::from_integer(BigInt::from(u8::from(
                        selected,
                    )))),
                    unknown: false,
                    source_ids: vec![party.entity_id.clone(), party.line_id.clone()],
                }
            } else {
                if self.q.context == "person"
                    && self
                        .registry
                        .entities
                        .iter()
                        .any(|e| e.id == party.entity_id && e.kind == "company")
                {
                    continue;
                }
                financial_entity_weight(
                    self.registry,
                    &party.entity_id,
                    &self.q.context,
                    self.q.person_id.as_deref(),
                    date,
                )?
            };
            self.sources.extend(w.source_ids);
            let weight = ratio(&w.weight)?;
            if w.unknown || (!weight.is_zero() && party.economic_share.is_none()) {
                self.warning("unknown_cashflow_debt_party",&terms.debt_id,"Les droits d’un côté visible du financement sont incomplets ; aucun flux de contrepartie n’a été supposé.");
                return Ok(None);
            }
            if let Some(share) = &party.economic_share {
                total += weight * parse_fraction(share)?;
            }
        }
        Ok(Some(total))
    }
}
fn asset_at_country(registry: &RegistrySnapshot, asset: &str, date: &str) -> Option<String> {
    apply_profile_periods(registry, date)
        .ok()?
        .assets
        .into_iter()
        .find(|a| a.id == asset)
        .map(|a| a.country)
}

/// P5 analytical receivables remain a proportional view. The central financial
/// report uses the explicit claim holders and does not transfer old arrears.
pub fn financial_rental_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    treasury: &TreasurySnapshot,
    debts: &DebtSnapshot,
    finance: &FinanceSnapshot,
    q: &RentalReportRequest,
) -> Result<RentalReport> {
    validate_finance(registry, properties, rentals, treasury, debts, finance)?;
    let settlements = non_cash_settlements(treasury)?;
    let mut out = rental_report_with_settlements(registry, properties, rentals, q, &settlements)?;
    out.warnings.retain(|w| w.code != "treasury_not_integrated");
    if let Some(coverage) = cover(finance, &q.date) {
        out.source_ids
            .extend([coverage.id.clone(), coverage.revision_id.clone()]);
    }
    if settlements.iter().any(|s| s.effective_on <= q.date) {
        out.warnings.push(RentalWarning{code:"non_cash_rent_settlement".into(),object_id:"scope".into(),message:"Les dépôts explicitement appliqués réduisent le dû et sa dette de restitution sans nouvel encaissement ni loyer reçu supplémentaire. Les impayés de cette vue restent analytiques ; le rapport financier utilise les créanciers explicites.".into()});
    }
    let Some(coverage) = cover(finance, &q.date)
        .filter(|c| c.assets_complete && c.debts_complete && c.rentals_complete)
    else {
        out.status = "partial".into();
        out.warnings.push(RentalWarning{code:"cashflow_coverage_missing".into(),object_id:"scope".into(),message:"Le cash-flow après financement exige un inventaire déclaré complet des actifs, dettes et locations à cette date, ainsi que des sources monétaires rapprochées.".into()});
        return Ok(out);
    };
    let mut flow = CashFlow {
        registry,
        properties,
        rentals,
        treasury,
        q,
        sources: BTreeSet::from([coverage.id.clone(), coverage.revision_id.clone()]),
        conversions: Vec::new(),
        warnings: Vec::new(),
    };
    let mut total = Some(BigRational::zero());
    for payment in &rentals.payments {
        if q.context == "person"
            && registry
                .entities
                .iter()
                .any(|e| e.id == payment.recipient_entity_id && e.kind == "company")
        {
            continue;
        }
        if q.context == "company"
            && q.person_id.as_deref() != Some(payment.recipient_entity_id.as_str())
        {
            continue;
        }
        let Some(asset) = flow.property_of_lease(&payment.lease_id) else {
            continue;
        };
        if !flow.selected(asset) || !flow.in_period(&payment.effective_on) {
            continue;
        }
        let cash = flow.source_cash(CashSource {
            kind: "rent_payment",
            id: &payment.id,
            revision: &payment.revision_id,
            amount: &parse_decimal(&payment.amount)?,
            date: &payment.effective_on,
            currency: &payment.currency,
            component: "full",
            side: "",
        })?;
        let allocated = rentals
            .allocations
            .iter()
            .filter(|a| a.payment_id == payment.id && a.effective_on <= q.date)
            .try_fold(BigRational::zero(), |sum, a| {
                Ok(sum + parse_decimal(&a.amount)?)
            })?;
        match (&mut total, cash) {
            (Some(total), Some(cash)) => {
                *total += cash * allocated / parse_decimal(&payment.amount)?
            }
            _ => total = None,
        }
    }
    for expense in &rentals.expenses {
        if !flow.selected(&expense.asset_id) {
            continue;
        }
        let Some(paid) = expense.paid_on.as_deref() else {
            if flow.in_period(&expense.effective_on) {
                total = None;
                flow.warning("unknown_expense_cash_date",&expense.id,"La date de décaissement d’une dépense de la période n’est pas renseignée ; elle ne peut pas être supposée impayée ou payée.");
            }
            continue;
        };
        if !flow.in_period(paid) {
            continue;
        }
        let cash = flow.source_cash(CashSource {
            kind: "rental_expense",
            id: &expense.id,
            revision: &expense.revision_id,
            amount: &-parse_decimal(&expense.amount)?,
            date: paid,
            currency: &expense.currency,
            component: "full",
            side: "",
        })?;
        match (&mut total, cash) {
            (Some(total), Some(cash)) => *total += cash,
            _ => total = None,
        }
    }
    // Debt service is attached analytically to financed assets, never inferred
    // from collateral or ownership. Every component keeps its one ledger link.
    for movement in debts
        .movements
        .iter()
        .filter(|m| m.kind == "principal_repayment")
    {
        if !flow.in_period(&movement.effective_on) {
            continue;
        }
        let links: Vec<_> = debts
            .asset_links
            .iter()
            .filter(|l| {
                l.debt_id == movement.debt_id
                    && l.relation_kind == "finances"
                    && applies(&l.valid_from, l.valid_to.as_deref(), &movement.effective_on)
                    && flow.selected(&l.asset_id)
            })
            .collect();
        if links.is_empty() {
            continue;
        }
        let mut fraction = BigRational::zero();
        let mut unknown = false;
        for link in links {
            flow.sources
                .extend([link.id.clone(), link.revision_id.clone()]);
            if let Some(share) = &link.attributed_principal_share {
                fraction += parse_fraction(share)?;
            } else {
                unknown = true;
            }
        }
        if unknown {
            total = None;
            flow.warning("unknown_cashflow_financing_allocation",&movement.id,"La fraction de service du financement affectée au bien est inconnue ; la propriété du bien ne la remplace pas.");
            continue;
        }
        if fraction.is_zero() {
            continue;
        }
        let contract = debts
            .contracts
            .iter()
            .find(|d| d.id == movement.debt_id)
            .ok_or_else(|| invalid("Contrat de financement absent."))?;
        flow.sources
            .extend([contract.id.clone(), contract.revision_id.clone()]);
        let Some(terms) = debts.terms.iter().find(|t| {
            t.debt_id == movement.debt_id
                && applies(&t.valid_from, t.valid_to.as_deref(), &movement.effective_on)
        }) else {
            total = None;
            flow.warning(
                "cashflow_debt_terms_missing",
                &movement.id,
                "Les parties du financement ne sont pas connues à la date du flux.",
            );
            continue;
        };
        for side in ["debtor", "creditor"] {
            let Some(visible) = flow.side_weight(terms, side, &movement.effective_on)? else {
                total = None;
                continue;
            };
            if visible.is_zero() {
                continue;
            }
            let sign = if side == "debtor" {
                BigInt::from(1)
            } else {
                BigInt::from(-1)
            };
            let mut components = vec![
                (
                    "principal",
                    parse_decimal(&movement.principal_change)? * &sign,
                ),
                ("interest", -parse_decimal(&movement.interest_paid)? * &sign),
            ];
            if side == "debtor" {
                components.push(("fee", -parse_decimal(&movement.fees_paid)?));
            }
            for (component, amount) in components {
                let cash = flow.source_cash(CashSource {
                    kind: "debt_movement",
                    id: &movement.id,
                    revision: &movement.revision_id,
                    amount: &amount,
                    date: &movement.effective_on,
                    currency: &contract.currency,
                    component,
                    side,
                })?;
                match (&mut total, cash) {
                    (Some(total), Some(cash)) => *total += cash * &fraction,
                    _ => total = None,
                }
            }
        }
    }
    out.cash_flow_after_financing_eur = total.as_ref().map(format_eur);
    if let Some(total) = total {
        out.exact_totals
            .insert("cash_flow_after_financing".into(), fraction_of(&total));
    } else {
        out.status = "partial".into();
    }
    out.source_ids.extend(flow.sources);
    out.source_ids.sort();
    out.source_ids.dedup();
    out.conversions.extend(flow.conversions);
    out.warnings.extend(flow.warnings);
    out.warnings.push(RentalWarning{code:"cashflow_account_scope".into(),object_id:"scope".into(),message:"Le cash-flow suit les comptes et leurs droits à la date des flux ; les avances et dépôts sont exclus des revenus. La réception déclarée d’un loyer et le flux économique détenu sur un compte joint peuvent donc différer. Les capitaux remboursés ne constituent pas un revenu locatif.".into()});
    if q.context == "person" {
        out.warnings.push(RentalWarning{code:"personal_cashflow_direct_accounts".into(),object_id:"scope".into(),message:"Dans la vue individuelle, le cash-flow présente les flux directs des comptes personnels ; les sommes retenues dans une société ne sont pas une distribution reçue personnellement.".into()});
    }
    if q.context == "company" {
        out.warnings.push(RentalWarning{code:"company_cashflow_direct_accounts".into(),object_id:"scope".into(),message:"Ce cash-flow porte sur les comptes propres de la société choisie, distincts du revenu analytique attribuable de ses participations.".into()});
    }
    Ok(out)
}
