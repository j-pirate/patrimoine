//! Exact local treasury, bank reconciliation and rental deposit sources.
use crate::*;
use num_rational::BigRational;
use serde::{Deserialize, Serialize};

macro_rules! treasury_record {
    ($row:ident,$input:ident,{$($field:ident:$ty:ty),*$(,)?}) => {
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row {pub id:String,pub revision_id:String,$(pub $field:$ty,)*}
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input {pub id:Option<String>,pub expected_revision_id:Option<String>,pub reason:String,$(pub $field:$ty,)*}
    }
}

treasury_record!(TreasuryAccount,SaveTreasuryAccountInput,{
    asset_id:String,kind:String,currency:String,tracking_start:String,
    legal_owner_entity_id:String,bank_name:String,reference:String,country:String,
    storage_location:String,notes:String,archived:bool,
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TreasuryEntry {
    pub line_id: String,
    pub account_id: String,
    pub amount: String,
    pub role: String,
}
treasury_record!(TreasuryTransaction,SaveTreasuryTransactionInput,{
    kind:String,effective_on:String,label:String,reference:String,notes:String,
    entries:Vec<TreasuryEntry>,
});
treasury_record!(AccountReconciliation,SaveAccountReconciliationInput,{
    account_id:String,effective_on:String,statement_balance:String,
    statement_reference:String,document_id:Option<String>,notes:String,
});
treasury_record!(CashLink,SaveCashLinkInput,{
    source_kind:String,source_id:String,source_revision_id:String,
    transaction_revision_id:String,line_id:String,amount:String,
    component:String,side:String,notes:String,
});
treasury_record!(DepositMovement,SaveDepositMovementInput,{
    lease_id:String,kind:String,amount:String,currency:String,effective_on:String,
    recipient_entity_id:String,due_id:Option<String>,component:String,notes:String,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreasurySnapshot {
    pub accounts: Vec<TreasuryAccount>,
    pub transactions: Vec<TreasuryTransaction>,
    pub reconciliations: Vec<AccountReconciliation>,
    pub cash_links: Vec<CashLink>,
    pub deposits: Vec<DepositMovement>,
    pub knowledge_sequence: u64,
}

/// Exactly one transaction selector is supplied. A new/corrected transaction
/// and its source/link are written under the same expected global sequence.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CashPostingInput {
    pub transaction: Option<SaveTreasuryTransactionInput>,
    pub existing_transaction_revision_id: Option<String>,
    pub line_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostedRentAllocation {
    pub due_id: String,
    pub component: String,
    pub amount: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostRentPaymentInput {
    pub expected_sequence: u64,
    pub payment: SaveRentPaymentInput,
    pub allocations: Vec<PostedRentAllocation>,
    pub cash: CashPostingInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostRentalExpenseInput {
    pub expected_sequence: u64,
    pub expense: SaveRentalExpenseInput,
    pub cash: CashPostingInput,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostDepositInput {
    pub expected_sequence: u64,
    pub movement: SaveDepositMovementInput,
    pub cash: Option<CashPostingInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreasuryReportRequest {
    pub account_id: String,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedFee {
    pub currency: String,
    pub amount: Fraction,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutedFx {
    pub transaction_id: String,
    pub revision_id: String,
    pub effective_on: String,
    pub from_currency: String,
    pub to_currency: String,
    pub debited_amount: String,
    pub credited_amount: String,
    pub exact_rate: Fraction,
    pub fees: Vec<ExecutedFee>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconciliationResult {
    pub reconciliation_id: String,
    pub revision_id: String,
    pub effective_on: String,
    pub statement_balance: String,
    pub calculated_balance: Option<String>,
    pub difference: Option<String>,
    pub exact_difference: Option<Fraction>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryReport {
    pub account_id: String,
    pub asset_id: String,
    pub date: String,
    pub knowledge_sequence: u64,
    pub currency: String,
    pub status: String,
    pub balance_original: Option<String>,
    pub exact_balance: Option<Fraction>,
    pub balance_eur: Option<String>,
    pub exact_balance_eur: Option<Fraction>,
    pub known_movement_total: Fraction,
    pub overdraft: bool,
    pub opening_transaction_id: Option<String>,
    pub executed_fx: Vec<ExecutedFx>,
    pub reconciliations: Vec<ReconciliationResult>,
    pub conversions: Vec<FxResult>,
    pub source_ids: Vec<String>,
    pub warnings: Vec<TreasuryWarning>,
}

/// Public exact helpers and validators follow the DTO declarations below.
pub type ExactTreasuryBalance = BigRational;

use crate::exact::{currency_minor_units, format_currency, format_eur, fraction_of, parse_decimal};
use crate::fx::{convert_currency_unchecked, converted_rational};
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
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(invalid("Un identifiant de trésorerie est invalide."));
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
        return Err(invalid("Une désignation de trésorerie est vide, trop longue ou contient un caractère interdit."));
    }
    Ok(())
}
fn warning(out: &mut Vec<TreasuryWarning>, code: &str, id: &str, message: &str) {
    if !out
        .iter()
        .any(|w| w.code == code && w.object_id == id && w.message == message)
    {
        out.push(TreasuryWarning {
            code: code.into(),
            object_id: id.into(),
            message: message.into(),
        });
    }
}
fn opening<'a>(s: &'a TreasurySnapshot, account: &str) -> Option<&'a TreasuryTransaction> {
    s.transactions
        .iter()
        .find(|t| t.kind == "opening" && t.entries.iter().any(|e| e.account_id == account))
}

/// Exact balance is unknown until an explicit opening is present. An opening
/// at zero is a source, not the absence of one. Dates are end-of-civil-day.
pub fn account_balance_at(
    s: &TreasurySnapshot,
    account_id: &str,
    date: &str,
) -> Result<Option<BigRational>> {
    validate_date(date)?;
    let account = s
        .accounts
        .iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| invalid("Le compte demandé est inconnu."))?;
    if date < account.tracking_start.as_str() || opening(s, account_id).is_none() {
        return Ok(None);
    }
    let value = s
        .transactions
        .iter()
        .filter(|t| t.effective_on.as_str() <= date)
        .flat_map(|t| &t.entries)
        .filter(|e| e.account_id == account_id)
        .try_fold(BigRational::zero(), |sum, e| {
            Ok(sum + parse_decimal(&e.amount)?)
        })?;
    Ok(Some(value))
}

/// The explicit deposit holder bears the recorded restitution obligation.
pub fn deposit_obligation_at(
    s: &TreasurySnapshot,
    lease_id: &str,
    holder: &str,
    date: &str,
) -> Result<BigRational> {
    validate_date(date)?;
    s.deposits
        .iter()
        .filter(|d| {
            d.lease_id == lease_id
                && d.recipient_entity_id == holder
                && d.effective_on.as_str() <= date
        })
        .try_fold(BigRational::zero(), |sum, d| {
            Ok(sum + parse_decimal(&d.amount)?)
        })
}
/// Positive amount settled without receiving a new bank payment.
pub fn deposit_applied_at(
    s: &TreasurySnapshot,
    due_id: &str,
    component: &str,
    date: &str,
) -> Result<BigRational> {
    validate_date(date)?;
    s.deposits
        .iter()
        .filter(|d| {
            d.kind == "applied_to_due"
                && d.due_id.as_deref() == Some(due_id)
                && d.component == component
                && d.effective_on.as_str() <= date
        })
        .try_fold(BigRational::zero(), |sum, d| {
            Ok(sum - parse_decimal(&d.amount)?)
        })
}

fn referenced_line<'a>(
    s: &'a TreasurySnapshot,
    link: &CashLink,
) -> Result<(
    &'a TreasuryTransaction,
    &'a TreasuryEntry,
    &'a TreasuryAccount,
)> {
    let transaction = s
        .transactions
        .iter()
        .find(|t| t.revision_id == link.transaction_revision_id)
        .ok_or_else(|| {
            invalid("Le rapprochement doit désigner la révision courante exacte d’une transaction.")
        })?;
    let entry = transaction
        .entries
        .iter()
        .find(|e| e.line_id == link.line_id)
        .ok_or_else(|| {
            invalid("La ligne de trésorerie rapprochée n’appartient pas à cette révision.")
        })?;
    let account = s
        .accounts
        .iter()
        .find(|a| a.id == entry.account_id)
        .ok_or_else(|| invalid("Le compte de la ligne rapprochée est inconnu."))?;
    Ok((transaction, entry, account))
}

fn validate_deposits(
    registry: &RegistrySnapshot,
    rentals: &RentalSnapshot,
    s: &TreasurySnapshot,
) -> Result<()> {
    let mut obligations: BTreeMap<(&str, &str, &str), BTreeMap<&str, BigRational>> =
        BTreeMap::new();
    for d in &s.deposits {
        let lease = rentals
            .leases
            .iter()
            .find(|l| l.id == d.lease_id)
            .ok_or_else(|| invalid("Le dépôt concerne un bail inconnu."))?;
        if !registry
            .entities
            .iter()
            .any(|e| e.id == d.recipient_entity_id)
        {
            return Err(invalid("Le détenteur explicite du dépôt est inconnu."));
        }
        validate_date(&d.effective_on)?;
        currency_minor_units(&d.currency)?;
        text(&d.notes, 10000, true)?;
        if !rentals
            .lease_terms
            .iter()
            .chain(&rentals.lease_term_revisions)
            .any(|t| t.lease_id == lease.id && t.currency == d.currency)
        {
            return Err(invalid(
                "La devise du dépôt ne correspond pas aux conditions connues de son bail.",
            ));
        }
        let amount = parse_decimal(&d.amount)?;
        if amount.is_zero() {
            return Err(invalid(
                "Un mouvement de dépôt doit avoir un montant non nul.",
            ));
        }
        match d.kind.as_str() {
            "received" if amount.is_positive() && d.due_id.is_none() && d.component.is_empty()=>{},
            "returned" if amount.is_negative() && d.due_id.is_none() && d.component.is_empty()=>{},
            "adjustment" if d.due_id.is_none() && d.component.is_empty()=>{},
            "applied_to_due" if amount.is_negative() && ["rent","charges"].contains(&d.component.as_str())=>{
                let due=rentals.dues.iter().find(|due|Some(&due.id)==d.due_id.as_ref()).ok_or_else(||invalid("La compensation doit désigner une échéance exacte."))?;
                if due.lease_id!=d.lease_id || due.currency!=d.currency || d.effective_on<due.due_on {return Err(invalid("La compensation est incompatible avec le bail, la devise ou l’exigibilité de l’échéance."));}
            },
            _=>return Err(invalid("Réception de dépôt positive ; restitution et compensation négatives ; seule la compensation désigne une échéance.")),
        }
        *obligations
            .entry((&d.lease_id, &d.recipient_entity_id, &d.currency))
            .or_default()
            .entry(&d.effective_on)
            .or_insert_with(BigRational::zero) += amount;
    }
    for dates in obligations.values() {
        let mut balance = BigRational::zero();
        for amount in dates.values() {
            balance += amount;
            if balance.is_negative() {
                return Err(invalid(
                    "Une restitution ou compensation dépasse le dépôt détenu à une date affectée.",
                ));
            }
        }
    }
    let mut settlements: BTreeMap<(&str, &str), BTreeMap<&str, BigRational>> = BTreeMap::new();
    for a in &rentals.allocations {
        *settlements
            .entry((&a.due_id, &a.component))
            .or_default()
            .entry(&a.effective_on)
            .or_insert_with(BigRational::zero) += parse_decimal(&a.amount)?;
    }
    for d in s.deposits.iter().filter(|d| d.kind == "applied_to_due") {
        let due = d
            .due_id
            .as_deref()
            .ok_or_else(|| invalid("Échéance de compensation absente."))?;
        *settlements
            .entry((due, &d.component))
            .or_default()
            .entry(&d.effective_on)
            .or_insert_with(BigRational::zero) -= parse_decimal(&d.amount)?;
    }
    for ((due, component), dates) in settlements {
        let due = rentals
            .dues
            .iter()
            .find(|d| d.id == due)
            .ok_or_else(|| invalid("Une affectation concerne une échéance inconnue."))?;
        let maximum = parse_decimal(if component == "rent" {
            &due.rent
        } else {
            &due.charges
        })?;
        let mut balance = BigRational::zero();
        for amount in dates.values() {
            balance += amount;
            if balance.is_negative() || balance > maximum {
                return Err(invalid("Paiements et dépôts appliqués dépassent une composante d’échéance à une date affectée."));
            }
        }
    }
    Ok(())
}

/// These snapshots must come from one read transaction at the same K.
pub fn validate_treasury(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    s: &TreasurySnapshot,
) -> Result<()> {
    validate_rentals(registry, properties, rentals)?;
    if s.knowledge_sequence != registry.knowledge_sequence {
        return Err(invalid(
            "La trésorerie et ses sources doivent utiliser les mêmes connaissances.",
        ));
    }
    let mut ids = HashSet::new();
    let mut revisions = HashSet::new();
    for (id, rev) in s
        .accounts
        .iter()
        .map(|r| (&r.id, &r.revision_id))
        .chain(s.transactions.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.reconciliations.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.cash_links.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.deposits.iter().map(|r| (&r.id, &r.revision_id)))
    {
        identifier(id)?;
        identifier(rev)?;
        if !ids.insert(id.as_str()) || !revisions.insert(rev.as_str()) {
            return Err(invalid(
                "Une identité ou une révision de trésorerie est dupliquée.",
            ));
        }
    }
    let mut asset_ids = HashSet::new();
    let accounts: HashMap<_, _> = s.accounts.iter().map(|a| (a.id.as_str(), a)).collect();
    for a in &s.accounts {
        if !registry.assets.iter().any(|asset| asset.id == a.asset_id)
            || properties
                .properties
                .iter()
                .any(|p| p.asset_id == a.asset_id)
            || !asset_ids.insert(&a.asset_id)
        {
            return Err(invalid("Un compte complète un actif existant unique et ne peut pas être aussi un bien immobilier."));
        }
        if !registry
            .entities
            .iter()
            .any(|e| e.id == a.legal_owner_entity_id)
        {
            return Err(invalid("Le titulaire juridique du compte est inconnu."));
        }
        if !["bank", "cash"].contains(&a.kind.as_str())
            || !["FR", "TN", "CH", "AE"].contains(&a.country.as_str())
        {
            return Err(invalid("Type ou pays de compte inconnu."));
        }
        currency_minor_units(&a.currency)?;
        validate_date(&a.tracking_start)?;
        text(&a.bank_name, 160, a.kind == "bank")?;
        text(&a.reference, 200, false)?;
        text(&a.storage_location, 1000, a.kind == "cash")?;
        text(&a.notes, 10000, false)?;
    }
    let mut openings = HashSet::new();
    for t in &s.transactions {
        validate_date(&t.effective_on)?;
        text(&t.label, 200, true)?;
        text(&t.reference, 1000, false)?;
        text(&t.notes, 10000, false)?;
        if ![
            "opening",
            "receipt",
            "payment",
            "transfer",
            "fx",
            "rent",
            "loan",
            "asset_trade",
            "dividend",
            "correction",
            "deposit",
        ]
        .contains(&t.kind.as_str())
            || t.entries.is_empty()
            || t.entries.len() > 1000
        {
            return Err(invalid(
                "Une transaction doit avoir un type connu et entre une et mille lignes.",
            ));
        }
        let mut line_ids = HashSet::new();
        let mut principal = Vec::new();
        let mut currencies = HashSet::new();
        for e in &t.entries {
            identifier(&e.line_id)?;
            if !line_ids.insert(&e.line_id) {
                return Err(invalid(
                    "Une ligne apparaît plusieurs fois dans la transaction.",
                ));
            }
            let a = accounts
                .get(e.account_id.as_str())
                .ok_or_else(|| invalid("Une ligne désigne un compte inconnu."))?;
            if t.effective_on < a.tracking_start {
                return Err(invalid(
                    "Une ligne ne peut précéder le début de suivi de son compte.",
                ));
            }
            let n = parse_decimal(&e.amount)?;
            if !["principal", "fee", "opening", "adjustment"].contains(&e.role.as_str())
                || (n.is_zero() && t.kind != "opening")
            {
                return Err(invalid(
                    "Le rôle de ligne est inconnu ou son montant nul hors ouverture.",
                ));
            }
            if e.role == "fee" && !n.is_negative() {
                return Err(invalid(
                    "Les frais de trésorerie sont un décaissement négatif explicite.",
                ));
            }
            if e.role == "opening" && t.kind != "opening" {
                return Err(invalid(
                    "Une ligne d’ouverture appartient seulement à une transaction d’ouverture.",
                ));
            }
            if e.role == "principal" {
                principal.push((e, a, n));
            }
            currencies.insert(&a.currency);
        }
        match t.kind.as_str() {
            "opening" => {
                let e = &t.entries[0];
                let a = accounts[e.account_id.as_str()];
                if t.entries.len() != 1
                    || e.role != "opening"
                    || t.effective_on != a.tracking_start
                    || !openings.insert(&a.id)
                {
                    return Err(invalid("Chaque compte a au plus une ouverture explicite, à sa date de début de suivi, avec une seule ligne."));
                }
            }
            "transfer" | "fx" => {
                if principal.len() != 2
                    || principal[0].0.account_id == principal[1].0.account_id
                    || t.entries
                        .iter()
                        .any(|e| !["principal", "fee"].contains(&e.role.as_str()))
                {
                    return Err(invalid("Un transfert ou change comporte deux comptes principaux distincts et des frais séparés."));
                }
                let debit = principal.iter().find(|(_, _, n)| n.is_negative());
                let credit = principal.iter().find(|(_, _, n)| n.is_positive());
                let (Some(debit), Some(credit)) = (debit, credit) else {
                    return Err(invalid(
                        "Un transfert ou change doit avoir un débit et un crédit principaux.",
                    ));
                };
                if t.kind == "transfer"
                    && (debit.1.currency != credit.1.currency
                        || &debit.2 + &credit.2 != BigRational::zero())
                {
                    return Err(invalid("Un transfert conserve la même devise et deux principaux exactement opposés, hors frais."));
                }
                if t.kind == "fx" && debit.1.currency == credit.1.currency {
                    return Err(invalid("Un change exécuté relie deux devises différentes."));
                }
            }
            "correction" => {
                if t.entries.len() != 1 || t.entries[0].role != "adjustment" {
                    return Err(invalid(
                        "Une correction est une ligne d’ajustement justifiée explicite.",
                    ));
                }
            }
            "receipt" => {
                if principal.len() != t.entries.len()
                    || principal.iter().any(|(_, _, n)| !n.is_positive())
                {
                    return Err(invalid(
                        "Une entrée simple contient seulement des principaux positifs.",
                    ));
                }
            }
            "payment" => {
                if t.entries
                    .iter()
                    .any(|e| !["principal", "fee"].contains(&e.role.as_str()))
                    || principal.is_empty()
                    || principal.iter().any(|(_, _, n)| !n.is_negative())
                {
                    return Err(invalid(
                        "Une sortie simple contient des principaux négatifs et des frais séparés.",
                    ));
                }
            }
            _ => {
                if currencies.len() != 1
                    || principal.is_empty()
                    || t.entries
                        .iter()
                        .any(|e| !["principal", "fee"].contains(&e.role.as_str()))
                {
                    return Err(invalid("Une opération métier conserve sa devise, ses principaux et ses frais séparés ; utilisez un change explicite entre devises."));
                }
                if principal.iter().any(|(_, _, n)| n.is_positive())
                    && principal.iter().any(|(_, _, n)| n.is_negative())
                    && principal.iter().map(|(_, _, n)| n).sum::<BigRational>()
                        != BigRational::zero()
                {
                    return Err(invalid("Les deux faces suivies d’une opération interne sont opposées, hors frais explicitement séparés."));
                }
            }
        }
    }
    // A physical cash box cannot acquire a negative known balance at any civil
    // boundary. Without an opening its balance is unknown, never assumed zero.
    for a in s
        .accounts
        .iter()
        .filter(|a| a.kind == "cash" && opening(s, &a.id).is_some())
    {
        let mut dates: BTreeMap<&str, BigRational> = BTreeMap::new();
        for t in &s.transactions {
            for e in t.entries.iter().filter(|e| e.account_id == a.id) {
                *dates
                    .entry(&t.effective_on)
                    .or_insert_with(BigRational::zero) += parse_decimal(&e.amount)?;
            }
        }
        let mut balance = BigRational::zero();
        for delta in dates.values() {
            balance += delta;
            if balance.is_negative() {
                return Err(invalid(
                    "Une caisse physique présente un solde négatif à une date affectée.",
                ));
            }
        }
    }
    for r in &s.reconciliations {
        let a = accounts
            .get(r.account_id.as_str())
            .ok_or_else(|| invalid("Le rapprochement concerne un compte inconnu."))?;
        validate_date(&r.effective_on)?;
        parse_decimal(&r.statement_balance)?;
        if r.effective_on < a.tracking_start {
            return Err(invalid("Le relevé précède le début de suivi du compte."));
        }
        text(&r.statement_reference, 1000, r.document_id.is_none())?;
        text(&r.notes, 10000, false)?;
        if let Some(id) = &r.document_id {
            identifier(id)?;
        }
    }
    validate_deposits(registry, rentals, s)?;
    validate_non_debt_cash_links(rentals, s)
}

fn validate_non_debt_cash_links(rentals: &RentalSnapshot, s: &TreasurySnapshot) -> Result<()> {
    let mut used_lines: HashMap<(&str, &str), BigRational> = HashMap::new();
    let mut used_sources: HashMap<(&str, &str, &str, &str), BigRational> = HashMap::new();
    for link in &s.cash_links {
        identifier(&link.source_id)?;
        identifier(&link.source_revision_id)?;
        text(&link.notes, 10000, false)?;
        if ![
            "rent_payment",
            "rental_expense",
            "deposit_movement",
            "debt_movement",
        ]
        .contains(&link.source_kind.as_str())
        {
            return Err(invalid("La nature de source rapprochée est inconnue."));
        }
        let (t, e, a) = referenced_line(s, link)?;
        if t.kind == "opening" {
            return Err(invalid("Un solde d’ouverture est un stock confirmé, pas un encaissement ou décaissement à rapprocher comme un flux."));
        }
        let n = parse_decimal(&link.amount)?;
        let cash = parse_decimal(&e.amount)?;
        if n.is_zero() || n.is_negative() != cash.is_negative() {
            return Err(invalid("Un rapprochement doit garder le signe de sa ligne de trésorerie et un montant non nul."));
        }
        *used_lines
            .entry((&link.transaction_revision_id, &link.line_id))
            .or_insert_with(BigRational::zero) += n.abs();
        let source_key = (
            link.source_kind.as_str(),
            link.source_id.as_str(),
            link.component.as_str(),
            link.side.as_str(),
        );
        *used_sources
            .entry(source_key)
            .or_insert_with(BigRational::zero) += n.abs();
        if link.source_kind == "debt_movement" {
            continue;
        }
        if link.component != "full" || !link.side.is_empty() {
            return Err(invalid(
                "Les sources locatives utilisent la composante full sans côté de financement.",
            ));
        }
        let (revision, currency, date, expected, recipient) = match link.source_kind.as_str() {
            "rent_payment" => {
                let p = rentals
                    .payments
                    .iter()
                    .find(|p| p.id == link.source_id)
                    .ok_or_else(|| invalid("Le paiement locatif rapproché est inconnu."))?;
                (
                    &p.revision_id,
                    &p.currency,
                    p.effective_on.as_str(),
                    parse_decimal(&p.amount)?,
                    Some(p.recipient_entity_id.as_str()),
                )
            }
            "rental_expense" => {
                let p = rentals
                    .expenses
                    .iter()
                    .find(|p| p.id == link.source_id)
                    .ok_or_else(|| invalid("La dépense rapprochée est inconnue."))?;
                let date=p.paid_on.as_deref().ok_or_else(||invalid("La dépense doit avoir une date de décaissement explicite avant rapprochement."))?;
                (
                    &p.revision_id,
                    &p.currency,
                    date,
                    -parse_decimal(&p.amount)?,
                    None,
                )
            }
            "deposit_movement" => {
                let d = s
                    .deposits
                    .iter()
                    .find(|d| d.id == link.source_id)
                    .ok_or_else(|| invalid("Le mouvement de dépôt rapproché est inconnu."))?;
                if !["received", "returned"].contains(&d.kind.as_str()) {
                    return Err(invalid("Une compensation ou correction d’obligation de dépôt ne crée pas un mouvement bancaire implicite."));
                }
                (
                    &d.revision_id,
                    &d.currency,
                    d.effective_on.as_str(),
                    parse_decimal(&d.amount)?,
                    Some(d.recipient_entity_id.as_str()),
                )
            }
            _ => unreachable!("validated source kind"),
        };
        if revision != &link.source_revision_id
            || currency != &a.currency
            || date != t.effective_on
            || n.is_negative() != expected.is_negative()
            || recipient.is_some_and(|id| id != a.legal_owner_entity_id)
        {
            return Err(invalid("Le rapprochement doit garder la révision source exacte, sa devise, sa date, son signe et son bénéficiaire explicite."));
        }
        if used_sources[&source_key] > expected.abs() {
            return Err(invalid(
                "Les rapprochements dépassent le montant de leur source financière.",
            ));
        }
    }
    for ((revision, line), used) in used_lines {
        let cash = s
            .transactions
            .iter()
            .find(|t| t.revision_id == revision)
            .and_then(|t| t.entries.iter().find(|e| e.line_id == line))
            .ok_or_else(|| invalid("Ligne de rapprochement absente."))?;
        if used > parse_decimal(&cash.amount)?.abs() {
            return Err(invalid(
                "Plusieurs sources dépassent la capacité d’une même ligne bancaire.",
            ));
        }
    }
    Ok(())
}

/// Complements the rental/deposit link validator with the two financing faces.
/// The borrower pays principal, interest and fees; the creditor receives only
/// principal and interest. Fees are not a second capital repayment.
pub fn validate_treasury_debt_links(
    registry: &RegistrySnapshot,
    s: &TreasurySnapshot,
    debts: &DebtSnapshot,
) -> Result<()> {
    validate_debts(registry, debts)?;
    if s.knowledge_sequence != debts.knowledge_sequence {
        return Err(invalid(
            "Trésorerie et financements doivent utiliser les mêmes connaissances.",
        ));
    }
    let mut used: BTreeMap<(&str, &str, &str), BigRational> = BTreeMap::new();
    for link in s
        .cash_links
        .iter()
        .filter(|l| l.source_kind == "debt_movement")
    {
        let m = debts
            .movements
            .iter()
            .find(|m| m.id == link.source_id)
            .ok_or_else(|| invalid("Le mouvement de financement rapproché est inconnu."))?;
        let contract = debts
            .contracts
            .iter()
            .find(|d| d.id == m.debt_id)
            .ok_or_else(|| invalid("Le contrat du mouvement rapproché est inconnu."))?;
        if !["drawdown", "principal_repayment"].contains(&m.kind.as_str())
            || m.revision_id != link.source_revision_id
        {
            return Err(invalid("Seule la révision courante d’un tirage ou remboursement réel peut être rapprochée ; ouverture et ajustements ne fabriquent pas de cash."));
        }
        let (t, _, account) = referenced_line(s, link)?;
        let terms = debts
            .terms
            .iter()
            .find(|d| {
                d.debt_id == m.debt_id
                    && applies(&d.valid_from, d.valid_to.as_deref(), &m.effective_on)
            })
            .ok_or_else(|| {
                invalid(
                    "Les parties du financement ne sont pas identifiées à la date du mouvement.",
                )
            })?;
        if !["creditor", "debtor"].contains(&link.side.as_str())
            || !terms
                .parties
                .iter()
                .any(|p| p.side == link.side && p.entity_id == account.legal_owner_entity_id)
        {
            return Err(invalid("Le titulaire du compte doit être une partie explicite du côté de financement rapproché."));
        }
        let principal = parse_decimal(&m.principal_change)?;
        let expected=match (link.component.as_str(),link.side.as_str()) {
            ("principal","debtor")=>principal,
            ("principal","creditor")=>-principal,
            ("interest","debtor")=>-parse_decimal(&m.interest_paid)?,
            ("interest","creditor")=>parse_decimal(&m.interest_paid)?,
            ("fee","debtor")=>-parse_decimal(&m.fees_paid)?,
            _=>return Err(invalid("La composante ou le côté de financement est invalide ; les frais ne sont pas une recette supposée du créancier.")),
        };
        let amount = parse_decimal(&link.amount)?;
        if expected.is_zero()
            || amount.is_zero()
            || amount.is_negative() != expected.is_negative()
            || account.currency != contract.currency
            || t.effective_on != m.effective_on
        {
            return Err(invalid("Le lien de financement doit garder le montant disponible, sa composante, son signe, sa devise et sa date."));
        }
        let key = (
            link.source_id.as_str(),
            link.component.as_str(),
            link.side.as_str(),
        );
        *used.entry(key).or_insert_with(BigRational::zero) += amount.abs();
        if used[&key] > expected.abs() {
            return Err(invalid(
                "Les rapprochements d’un côté dépassent la composante réelle du financement.",
            ));
        }
    }
    Ok(())
}

fn executed_fx(s: &TreasurySnapshot, t: &TreasuryTransaction) -> Result<ExecutedFx> {
    let mut principal = t.entries.iter().filter(|e| e.role == "principal");
    let a = principal
        .next()
        .ok_or_else(|| invalid("Débit de change absent."))?;
    let b = principal
        .next()
        .ok_or_else(|| invalid("Crédit de change absent."))?;
    let (debit, credit) = if parse_decimal(&a.amount)?.is_negative() {
        (a, b)
    } else {
        (b, a)
    };
    let from = s
        .accounts
        .iter()
        .find(|a| a.id == debit.account_id)
        .ok_or_else(|| invalid("Compte débité inconnu."))?;
    let to = s
        .accounts
        .iter()
        .find(|a| a.id == credit.account_id)
        .ok_or_else(|| invalid("Compte crédité inconnu."))?;
    let mut fees: BTreeMap<&str, BigRational> = BTreeMap::new();
    for e in t.entries.iter().filter(|e| e.role == "fee") {
        let account = s
            .accounts
            .iter()
            .find(|a| a.id == e.account_id)
            .ok_or_else(|| invalid("Compte des frais inconnu."))?;
        *fees
            .entry(&account.currency)
            .or_insert_with(BigRational::zero) += parse_decimal(&e.amount)?.abs();
    }
    Ok(ExecutedFx {
        transaction_id: t.id.clone(),
        revision_id: t.revision_id.clone(),
        effective_on: t.effective_on.clone(),
        from_currency: from.currency.clone(),
        to_currency: to.currency.clone(),
        debited_amount: debit.amount.trim_start_matches('-').into(),
        credited_amount: credit.amount.clone(),
        exact_rate: fraction_of(&(parse_decimal(&credit.amount)? / -parse_decimal(&debit.amount)?)),
        fees: fees
            .into_iter()
            .map(|(currency, n)| ExecutedFee {
                currency: currency.into(),
                amount: fraction_of(&n),
            })
            .collect(),
    })
}

pub fn treasury_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    rentals: &RentalSnapshot,
    s: &TreasurySnapshot,
    q: &TreasuryReportRequest,
) -> Result<TreasuryReport> {
    validate_treasury(registry, properties, rentals, s)?;
    validate_date(&q.date)?;
    if q.knowledge_sequence
        .is_some_and(|k| k != s.knowledge_sequence)
    {
        return Err(invalid(
            "La borne de connaissance ne correspond pas aux sources de trésorerie.",
        ));
    }
    let account = s
        .accounts
        .iter()
        .find(|a| a.id == q.account_id)
        .ok_or_else(|| invalid("Le compte demandé est inconnu."))?;
    let balance = account_balance_at(s, &account.id, &q.date)?;
    let mut warnings = Vec::new();
    let mut sources = BTreeSet::from([
        account.id.clone(),
        account.revision_id.clone(),
        account.asset_id.clone(),
    ]);
    let mut known = BigRational::zero();
    let mut executions = Vec::new();
    for t in s.transactions.iter().filter(|t| {
        t.effective_on <= q.date && t.entries.iter().any(|e| e.account_id == account.id)
    }) {
        sources.extend([t.id.clone(), t.revision_id.clone()]);
        for e in t.entries.iter().filter(|e| e.account_id == account.id) {
            known += parse_decimal(&e.amount)?;
            sources.insert(e.line_id.clone());
        }
        if t.kind == "fx" {
            executions.push(executed_fx(s, t)?);
        }
    }
    let mut conversions = Vec::new();
    let mut eur = None;
    if let Some(balance) = &balance {
        let rate = convert_currency_unchecked(
            registry,
            &FxQuery {
                amount: "1".into(),
                from_currency: account.currency.clone(),
                to_currency: "EUR".into(),
                date: q.date.clone(),
                knowledge_sequence: Some(s.knowledge_sequence),
            },
        )?;
        for w in &rate.warnings {
            warning(&mut warnings, &w.code, &account.id, &w.message);
        }
        for leg in &rate.legs {
            sources.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
        }
        eur = converted_rational(&rate).map(|rate| balance * rate);
        conversions.push(rate);
    } else if q.date < account.tracking_start {
        warning(
            &mut warnings,
            "account_not_started",
            &account.id,
            "La date demandée précède le début du suivi ; aucun solde antérieur n’est supposé.",
        );
    } else {
        warning(&mut warnings,"missing_account_opening",&account.id,"Aucune ouverture confirmée n’est connue. Le total des mouvements connus ne constitue pas un solde complet.");
    }
    let overdraft = balance.as_ref().is_some_and(BigRational::is_negative);
    if overdraft {
        warning(&mut warnings,"bank_overdraft",&account.id,"Le solde négatif est un découvert. La consolidation le présente comme exposition débitrice, sans dupliquer un contrat de dette.");
    }
    let mut reconciliations = Vec::new();
    for r in s
        .reconciliations
        .iter()
        .filter(|r| r.account_id == account.id && r.effective_on <= q.date)
    {
        let calculated = account_balance_at(s, &account.id, &r.effective_on)?;
        let difference = calculated
            .as_ref()
            .map(|n| parse_decimal(&r.statement_balance).map(|statement| statement - n))
            .transpose()?;
        if difference.as_ref().is_some_and(|d| !d.is_zero()) {
            warning(&mut warnings,"reconciliation_difference",&r.id,"Le relevé diffère du solde calculé. Aucun mouvement de correction n’a été créé automatiquement.");
        }
        sources.extend([r.id.clone(), r.revision_id.clone()]);
        if let Some(doc) = &r.document_id {
            sources.insert(doc.clone());
        }
        reconciliations.push(ReconciliationResult {
            reconciliation_id: r.id.clone(),
            revision_id: r.revision_id.clone(),
            effective_on: r.effective_on.clone(),
            statement_balance: r.statement_balance.clone(),
            calculated_balance: calculated
                .as_ref()
                .map(|n| format_currency(n, &account.currency))
                .transpose()?,
            difference: difference
                .as_ref()
                .map(|n| format_currency(n, &account.currency))
                .transpose()?,
            exact_difference: difference.as_ref().map(fraction_of),
        });
    }
    for l in s.cash_links.iter().filter(|l| {
        s.transactions.iter().any(|t| {
            t.revision_id == l.transaction_revision_id
                && t.effective_on <= q.date
                && t.entries
                    .iter()
                    .any(|e| e.line_id == l.line_id && e.account_id == account.id)
        })
    }) {
        sources.extend([
            l.id.clone(),
            l.revision_id.clone(),
            l.source_id.clone(),
            l.source_revision_id.clone(),
        ]);
    }
    Ok(TreasuryReport {
        account_id: account.id.clone(),
        asset_id: account.asset_id.clone(),
        date: q.date.clone(),
        knowledge_sequence: s.knowledge_sequence,
        currency: account.currency.clone(),
        status: if balance.is_some() && eur.is_some() {
            "complete"
        } else {
            "partial"
        }
        .into(),
        balance_original: balance
            .as_ref()
            .map(|n| format_currency(n, &account.currency))
            .transpose()?,
        exact_balance: balance.as_ref().map(fraction_of),
        balance_eur: eur.as_ref().map(format_eur),
        exact_balance_eur: eur.as_ref().map(fraction_of),
        known_movement_total: fraction_of(&known),
        overdraft,
        opening_transaction_id: opening(s, &account.id)
            .filter(|t| t.effective_on <= q.date)
            .map(|t| t.id.clone()),
        executed_fx: executions,
        reconciliations,
        conversions,
        source_ids: sources.into_iter().collect(),
        warnings,
    })
}
