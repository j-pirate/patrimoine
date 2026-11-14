//! Identified physical lots, exact fine weights and proportional historical costs.
use crate::*;
use num_rational::BigRational;
use serde::{Deserialize, Serialize};

macro_rules! metal_record {
    ($row:ident,$input:ident,{$($field:ident:$ty:ty),*$(,)?}) => {
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row {pub id:String,pub revision_id:String,$(pub $field:$ty,)*}
        #[derive(Debug,Clone,Serialize,Deserialize,PartialEq,Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input {pub id:Option<String>,pub expected_revision_id:Option<String>,pub reason:String,$(pub $field:$ty,)*}
    }
}

metal_record!(MetalLot,SaveMetalLotInput,{
    asset_id:String,metal:String,form:String,designation:String,unit_weight:String,
    weight_unit:String,purity:String,tracking_start:String,cost_currency:String,
    legal_owner_entity_id:String,country:String,storage_location:String,
    valuation_basis:String,notes:String,archived:bool,
});
metal_record!(MetalMovement,SaveMetalMovementInput,{
    lot_id:String,kind:String,effective_on:String,quantity_change:String,
    cost_amount:Option<String>,trade_amount:Option<String>,trade_fees:String,
    trade_currency:Option<String>,cash_entity_id:String,counterparty_entity_id:Option<String>,
    country:String,storage_location:String,unlinked_reason:String,notes:String,
});
metal_record!(MetalQuote,SaveMetalQuoteInput,{
    metal:String,effective_on:String,price:String,currency:String,weight_unit:String,
    source:String,notes:String,
});
metal_record!(MetalPremium,SaveMetalPremiumInput,{
    lot_id:String,effective_on:String,kind:String,value:String,currency:String,
    discount_per_unit:String,source:String,notes:String,
});
metal_record!(MetalCashLink,SaveMetalCashLinkInput,{
    movement_id:String,movement_revision_id:String,transaction_revision_id:String,
    line_id:String,amount:String,notes:String,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetalSnapshot {
    pub lots: Vec<MetalLot>,
    pub movements: Vec<MetalMovement>,
    pub quotes: Vec<MetalQuote>,
    pub premiums: Vec<MetalPremium>,
    pub cash_links: Vec<MetalCashLink>,
    pub knowledge_sequence: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostedMetalCashLink {
    pub line_id: String,
    pub amount: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PostMetalMovementInput {
    pub expected_sequence: u64,
    pub movement: SaveMetalMovementInput,
    pub transaction: Option<SaveTreasuryTransactionInput>,
    pub existing_transaction_revision_id: Option<String>,
    pub links: Vec<PostedMetalCashLink>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferMetalOwnershipInput {
    pub expected_sequence: u64,
    pub lot_id: String,
    pub effective_on: String,
    pub ownerships: Vec<SaveOwnershipInput>,
    pub reason: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MetalReportRequest {
    pub lot_id: String,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MetalWarning {
    pub code: String,
    pub object_id: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalCostAllocation {
    pub movement_id: String,
    pub revision_id: String,
    pub quantity: Fraction,
    pub exact_cost_original: Option<Fraction>,
    pub exact_cost_eur: Option<Fraction>,
    pub net_proceeds_eur: Option<Fraction>,
    pub realized_gain_eur: Option<Fraction>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetalReport {
    pub lot_id: String,
    pub asset_id: String,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub valuation_basis: String,
    pub quantity_remaining: Option<String>,
    pub exact_quantity: Option<Fraction>,
    pub fine_weight_g: Option<String>,
    pub exact_fine_weight_g: Option<Fraction>,
    pub cost_currency: String,
    pub currency: String,
    pub cost_remaining_original: Option<String>,
    pub exact_cost_remaining_original: Option<Fraction>,
    pub cost_remaining_eur: Option<String>,
    pub exact_cost_remaining_eur: Option<Fraction>,
    pub quote: Option<MetalQuote>,
    pub premium: Option<MetalPremium>,
    pub intrinsic_original: Option<String>,
    pub exact_intrinsic_original: Option<Fraction>,
    pub intrinsic_eur: Option<String>,
    pub exact_intrinsic_eur: Option<Fraction>,
    pub premium_eur: Option<String>,
    pub premium_original: Option<String>,
    pub exact_premium_original: Option<Fraction>,
    pub exact_premium_eur: Option<Fraction>,
    pub discount_eur: Option<String>,
    pub discount_original: Option<String>,
    pub exact_discount_original: Option<Fraction>,
    pub exact_discount_eur: Option<Fraction>,
    pub resale_eur: Option<String>,
    pub resale_original: Option<String>,
    pub exact_resale_original: Option<Fraction>,
    pub exact_resale_eur: Option<Fraction>,
    pub valuation_eur: Option<String>,
    pub exact_valuation_eur: Option<Fraction>,
    pub realized_gain_eur: Option<String>,
    pub exact_realized_gain_eur: Option<Fraction>,
    pub unrealized_gain_eur: Option<String>,
    pub exact_unrealized_gain_eur: Option<Fraction>,
    pub country: String,
    pub storage_location: String,
    pub cost_allocations: Vec<MetalCostAllocation>,
    pub conversions: Vec<FxResult>,
    pub source_ids: Vec<String>,
    pub warnings: Vec<MetalWarning>,
}

use crate::exact::{
    currency_minor_units, format_currency, format_eur, fraction_of, parse_decimal, parse_rate,
};
use crate::fx::convert_currency_unchecked;
use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

fn invalid(message: &str) -> AppError {
    AppError::new("validation", message)
}
fn text(value: &str, maximum: usize, required: bool) -> Result<()> {
    if (required && value.trim().is_empty())
        || value.chars().count() > maximum
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid(
            "Un libellé de métal est vide, trop long ou contient un caractère interdit.",
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
        return Err(invalid("Un identifiant de métal est invalide."));
    }
    Ok(())
}
fn country(value: &str) -> Result<()> {
    if !["FR", "TN", "CH", "AE"].contains(&value) {
        return Err(invalid("Le pays du lot est inconnu."));
    }
    Ok(())
}
fn rational(value: &Fraction) -> Result<BigRational> {
    let n = value
        .num
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction calculée invalide."))?;
    let d = value
        .den
        .parse::<BigInt>()
        .map_err(|_| invalid("Fraction calculée invalide."))?;
    if d <= BigInt::zero() {
        return Err(invalid("Fraction calculée invalide."));
    }
    Ok(BigRational::new(n, d))
}
fn nonnegative(value: &str) -> Result<BigRational> {
    let n = parse_decimal(value)?;
    if n.is_negative() {
        return Err(invalid(
            "Le montant doit être positif ou explicitement nul.",
        ));
    }
    Ok(n)
}
/// Physical weight units; the troy ounce is exactly 31.1034768 grams.
pub fn metal_weight_in_grams(amount: &str, unit: &str) -> Result<BigRational> {
    let value = parse_decimal(amount)?;
    if value.is_negative() {
        return Err(invalid("Un poids ne peut pas être négatif."));
    }
    let factor = match unit {
        "g" => BigRational::one(),
        "kg" => BigRational::from_integer(BigInt::from(1000)),
        "troy_oz" => parse_decimal("31.1034768")?,
        _ => return Err(invalid("L’unité doit être g, kg ou once troy explicite.")),
    };
    Ok(value * factor)
}
/// This formatter is only used for finite physical decimal quantities. Financial
/// cost ratios keep a Fraction and are rounded once in the currency formatter.
fn physical_decimal(value: &BigRational) -> String {
    if value.is_zero() {
        return "0".into();
    }
    let negative = value.is_negative();
    let numerator = value.numer().abs();
    let denominator = value.denom();
    let mut out = (&numerator / denominator).to_string();
    let mut remainder = numerator % denominator;
    if !remainder.is_zero() {
        out.push('.');
        while !remainder.is_zero() {
            remainder *= 10;
            out.push_str(&(&remainder / denominator).to_string());
            remainder %= denominator;
        }
    }
    if negative {
        out.insert(0, '-');
    }
    out
}
fn movement_rank(m: &MetalMovement) -> u8 {
    match m.kind.as_str() {
        "opening" => 0,
        "purchase" => 1,
        "inventory_adjustment" if !m.quantity_change.starts_with('-') => 2,
        "sale" => 3,
        "inventory_adjustment" => 4,
        _ => 5,
    }
}
fn movements<'a>(s: &'a MetalSnapshot, lot: &str, date: &str) -> Vec<&'a MetalMovement> {
    let mut rows: Vec<_> = s
        .movements
        .iter()
        .filter(|m| m.lot_id == lot && m.effective_on.as_str() <= date)
        .collect();
    rows.sort_by(|a, b| {
        (&a.effective_on, movement_rank(a), &a.id).cmp(&(&b.effective_on, movement_rank(b), &b.id))
    });
    rows
}
/// The actual signed cash paid or received by a trade. Openings, inventory and
/// storage changes deliberately have no inferred bank counterpart.
pub fn metal_movement_cash_amount(m: &MetalMovement) -> Result<Option<BigRational>> {
    if !["purchase", "sale"].contains(&m.kind.as_str()) {
        return Ok(None);
    }
    let amount = nonnegative(
        m.trade_amount
            .as_deref()
            .ok_or_else(|| invalid("Le prix réellement payé ou reçu manque."))?,
    )?;
    let fee = nonnegative(&m.trade_fees)?;
    Ok(Some(if m.kind == "purchase" {
        -amount - fee
    } else {
        amount - fee
    }))
}
pub fn metal_cash_coverage(s: &MetalSnapshot, m: &MetalMovement) -> Result<bool> {
    let Some(expected) = metal_movement_cash_amount(m)? else {
        return Ok(true);
    };
    let mut linked = BigRational::zero();
    for link in s
        .cash_links
        .iter()
        .filter(|l| l.movement_id == m.id && l.movement_revision_id == m.revision_id)
    {
        linked += parse_decimal(&link.amount)?;
    }
    Ok(linked == expected)
}
/// Exact stock is unknown until the lot's explicit opening has been recorded.
pub fn metal_quantity_at(
    s: &MetalSnapshot,
    lot_id: &str,
    date: &str,
) -> Result<Option<BigRational>> {
    validate_date(date)?;
    let lot = s
        .lots
        .iter()
        .find(|l| l.id == lot_id)
        .ok_or_else(|| invalid("Lot de métal inconnu."))?;
    if date < lot.tracking_start.as_str()
        || !s
            .movements
            .iter()
            .any(|m| m.lot_id == lot_id && m.kind == "opening" && m.effective_on.as_str() <= date)
    {
        return Ok(None);
    }
    let mut total = BigRational::zero();
    for m in movements(s, lot_id, date) {
        total += parse_decimal(&m.quantity_change)?;
    }
    Ok(Some(total))
}

pub fn validate_metals(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    treasury: &TreasurySnapshot,
    s: &MetalSnapshot,
) -> Result<()> {
    if [
        properties.knowledge_sequence,
        treasury.knowledge_sequence,
        s.knowledge_sequence,
    ]
    .iter()
    .any(|k| *k != registry.knowledge_sequence)
    {
        return Err(invalid(
            "Les sources de métal ne partagent pas la même connaissance.",
        ));
    }
    let mut ids = HashSet::new();
    let mut revisions = HashSet::new();
    for (id, revision) in s
        .lots
        .iter()
        .map(|r| (&r.id, &r.revision_id))
        .chain(s.movements.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.quotes.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.premiums.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(s.cash_links.iter().map(|r| (&r.id, &r.revision_id)))
    {
        identifier(id)?;
        identifier(revision)?;
        if !ids.insert(id) || !revisions.insert(revision) {
            return Err(invalid(
                "Une identité ou révision de métal apparaît plusieurs fois.",
            ));
        }
    }
    let mut assets = HashSet::new();
    let lots: HashMap<_, _> = s.lots.iter().map(|l| (l.id.as_str(), l)).collect();
    for lot in &s.lots {
        if !registry.assets.iter().any(|a| a.id == lot.asset_id)
            || !assets.insert(&lot.asset_id)
            || properties
                .properties
                .iter()
                .any(|p| p.asset_id == lot.asset_id)
            || treasury.accounts.iter().any(|a| a.asset_id == lot.asset_id)
        {
            return Err(invalid(
                "Un lot complète un actif unique existant, distinct d’un immeuble ou d’un compte.",
            ));
        }
        if !registry
            .entities
            .iter()
            .any(|e| e.id == lot.legal_owner_entity_id)
        {
            return Err(invalid("Le détenteur juridique du lot est inconnu."));
        }
        if !["gold", "silver"].contains(&lot.metal.as_str())
            || !["bar", "ingot", "coin", "other"].contains(&lot.form.as_str())
            || !["intrinsic", "resale"].contains(&lot.valuation_basis.as_str())
        {
            return Err(invalid(
                "Le métal, la forme ou la méthode de valorisation du lot est inconnu.",
            ));
        }
        if metal_weight_in_grams(&lot.unit_weight, &lot.weight_unit)?.is_zero() {
            return Err(invalid("Le poids unitaire doit être strictement positif."));
        }
        let purity = parse_decimal(&lot.purity)?;
        if purity <= BigRational::zero() || purity > BigRational::one() {
            return Err(invalid("La pureté doit être un ratio décimal strictement positif et inférieur ou égal à 1."));
        }
        validate_date(&lot.tracking_start)?;
        currency_minor_units(&lot.cost_currency)?;
        country(&lot.country)?;
        text(&lot.designation, 200, true)?;
        text(&lot.storage_location, 1000, true)?;
        text(&lot.notes, 10000, false)?;
    }
    let mut openings = HashSet::new();
    let mut location_dates = HashSet::new();
    for m in &s.movements {
        let lot = lots
            .get(m.lot_id.as_str())
            .ok_or_else(|| invalid("Un mouvement réfère un lot inconnu."))?;
        validate_date(&m.effective_on)?;
        if m.effective_on < lot.tracking_start {
            return Err(invalid("Un mouvement précède le début de suivi du lot."));
        }
        country(&m.country)?;
        text(&m.storage_location, 1000, true)?;
        text(&m.notes, 10000, false)?;
        text(&m.unlinked_reason, 2000, false)?;
        let qty = parse_decimal(&m.quantity_change)?;
        let fees = nonnegative(&m.trade_fees)?;
        if let Some(cost) = &m.cost_amount {
            nonnegative(cost)?;
        }
        if let Some(counterparty) = &m.counterparty_entity_id {
            if !registry.entities.iter().any(|e| e.id == *counterparty) {
                return Err(invalid("La contrepartie du métal est inconnue."));
            }
        }
        match m.kind.as_str() {
            "opening" => {
                if qty.is_negative()
                    || m.effective_on != lot.tracking_start
                    || !openings.insert(&m.lot_id)
                {
                    return Err(invalid("L’ouverture du lot doit être unique, non négative et à la date du début de suivi."));
                }
                if qty.is_zero()
                    && m.cost_amount
                        .as_ref()
                        .map(|c| nonnegative(c))
                        .transpose()?
                        .is_some_and(|c| !c.is_zero())
                {
                    return Err(invalid("Une ouverture de quantité nulle ne peut pas porter un coût de stock non nul."));
                }
            }
            "purchase" => {
                if qty <= BigRational::zero() {
                    return Err(invalid(
                        "Un achat doit ajouter une quantité strictement positive.",
                    ));
                }
            }
            "sale" => {
                if qty >= BigRational::zero() || m.cost_amount.is_some() {
                    return Err(invalid("Une vente doit retirer du stock ; son coût est alloué au prorata du lot, jamais saisi de nouveau."));
                }
            }
            "inventory_adjustment" => {
                if qty.is_zero() || (qty.is_negative() && m.cost_amount.is_some()) {
                    return Err(invalid("Une correction d’inventaire a une quantité non nulle et une sortie conserve le coût proportionnel."));
                }
            }
            "storage_transfer" => {
                if !qty.is_zero()
                    || m.cost_amount.is_some()
                    || !location_dates.insert((&m.lot_id, &m.effective_on))
                {
                    return Err(invalid("Un transfert de lieu porte sur le lot entier sans quantité ni coût et reste unique à sa date."));
                }
            }
            _ => return Err(invalid("Nature de mouvement métal inconnue.")),
        }
        let trade = ["purchase", "sale"].contains(&m.kind.as_str());
        if trade {
            metal_movement_cash_amount(m)?;
            currency_minor_units(
                m.trade_currency
                    .as_deref()
                    .ok_or_else(|| invalid("La devise du paiement manque."))?,
            )?;
            if !registry.entities.iter().any(|e| e.id == m.cash_entity_id) {
                return Err(invalid(
                    "Le payeur ou bénéficiaire de trésorerie du métal est inconnu.",
                ));
            }
        } else if m.trade_amount.is_some()
            || m.trade_currency.is_some()
            || !fees.is_zero()
            || !m.cash_entity_id.is_empty()
            || m.counterparty_entity_id.is_some()
        {
            return Err(invalid("Une ouverture, un inventaire ou un lieu ne crée pas de prix, frais ni contrepartie bancaire fictifs."));
        }
    }
    for lot in &s.lots {
        let mut quantity = BigRational::zero();
        let mut opened = false;
        let mut current_country = lot.country.as_str();
        let mut current_storage = lot.storage_location.as_str();
        for m in movements(s, &lot.id, "9999-12-31") {
            let change = parse_decimal(&m.quantity_change)?;
            if m.kind == "opening" {
                opened = true;
            }
            if change.is_negative() && !opened {
                return Err(invalid("La sortie de métal exige une ouverture confirmée."));
            }
            quantity += change;
            if quantity.is_negative() {
                return Err(invalid(
                    "Un mouvement rendrait le stock de métal négatif à une date historique.",
                ));
            }
            if m.kind == "storage_transfer" {
                current_country = &m.country;
                current_storage = &m.storage_location;
            } else if m.country != current_country || m.storage_location != current_storage {
                return Err(invalid("Le lieu d’un mouvement doit être le lieu suivi du lot ; un déplacement du lot entier exige un transfert de stockage explicite."));
            }
        }
    }
    for quote in &s.quotes {
        if !["gold", "silver"].contains(&quote.metal.as_str()) {
            return Err(invalid("Métal de la cotation inconnu."));
        }
        validate_date(&quote.effective_on)?;
        parse_rate(&quote.price)?;
        currency_minor_units(&quote.currency)?;
        metal_weight_in_grams("1", &quote.weight_unit)?;
        text(&quote.source, 2000, true)?;
        text(&quote.notes, 10000, false)?;
    }
    for premium in &s.premiums {
        let lot = lots
            .get(premium.lot_id.as_str())
            .ok_or_else(|| invalid("Prime d’un lot inconnu."))?;
        if !["amount_per_unit", "fraction_of_intrinsic"].contains(&premium.kind.as_str()) {
            return Err(invalid("Nature de prime inconnue."));
        }
        validate_date(&premium.effective_on)?;
        if premium.effective_on < lot.tracking_start {
            return Err(invalid("La prime précède le suivi du lot."));
        }
        nonnegative(&premium.value)?;
        nonnegative(&premium.discount_per_unit)?;
        currency_minor_units(&premium.currency)?;
        text(&premium.source, 2000, true)?;
        text(&premium.notes, 10000, false)?;
    }
    validate_metal_cash_links(registry, treasury, s)
}

pub fn validate_metal_cash_links(
    registry: &RegistrySnapshot,
    treasury: &TreasurySnapshot,
    s: &MetalSnapshot,
) -> Result<()> {
    let mut capacities: BTreeMap<(&str, &str), BigRational> = BTreeMap::new();
    for link in &treasury.cash_links {
        *capacities
            .entry((&link.transaction_revision_id, &link.line_id))
            .or_default() += parse_decimal(&link.amount)?.abs();
    }
    let mut linked: BTreeMap<&str, (BigRational, BigRational)> = BTreeMap::new();
    let mut duplicates = HashSet::new();
    for link in &s.cash_links {
        let m = s
            .movements
            .iter()
            .find(|m| m.id == link.movement_id && m.revision_id == link.movement_revision_id)
            .ok_or_else(|| {
                invalid("Le lien métal ne vise pas la révision courante exacte du mouvement.")
            })?;
        if metal_movement_cash_amount(m)?.is_none() {
            return Err(invalid(
                "Seul un achat ou une vente réelle peut être rapproché en banque.",
            ));
        }
        let transaction = treasury
            .transactions
            .iter()
            .find(|t| t.revision_id == link.transaction_revision_id)
            .ok_or_else(|| invalid("Révision exacte de journal métal absente."))?;
        let line = transaction
            .entries
            .iter()
            .find(|e| e.line_id == link.line_id)
            .ok_or_else(|| invalid("La ligne bancaire du métal est absente."))?;
        if transaction.kind == "opening" || transaction.effective_on != m.effective_on {
            return Err(invalid(
                "La ligne de trésorerie du métal doit être un flux réel à la même date.",
            ));
        }
        let account = treasury
            .accounts
            .iter()
            .find(|a| a.id == line.account_id)
            .ok_or_else(|| invalid("Compte du mouvement métal inconnu."))?;
        if account.currency != m.trade_currency.as_deref().unwrap_or("")
            || account.legal_owner_entity_id != m.cash_entity_id
            || !registry.entities.iter().any(|e| e.id == m.cash_entity_id)
        {
            return Err(invalid(
                "Le détenteur ou la devise du compte ne correspond pas au flux métal.",
            ));
        }
        let amount = parse_decimal(&link.amount)?;
        let line_amount = parse_decimal(&line.amount)?;
        if amount.is_zero() || amount.is_positive() != line_amount.is_positive() {
            return Err(invalid(
                "Le lien métal doit avoir le signe non nul de sa ligne bancaire.",
            ));
        }
        if !duplicates.insert((
            &link.movement_id,
            &link.transaction_revision_id,
            &link.line_id,
        )) {
            return Err(invalid(
                "La même ligne bancaire est reliée deux fois au même mouvement métal.",
            ));
        }
        text(&link.notes, 10000, false)?;
        *capacities
            .entry((&link.transaction_revision_id, &link.line_id))
            .or_default() += amount.abs();
        let totals = linked.entry(&m.id).or_default();
        if amount.is_positive() {
            totals.0 += amount;
        } else {
            totals.1 -= amount;
        }
    }
    for ((revision, line), total) in capacities {
        if let Some(entry) = treasury
            .transactions
            .iter()
            .find(|t| t.revision_id == revision)
            .and_then(|t| t.entries.iter().find(|e| e.line_id == line))
        {
            if total > parse_decimal(&entry.amount)?.abs() {
                return Err(invalid(
                    "Les sources rapprochées dépassent la capacité de leur ligne bancaire.",
                ));
            }
        }
    }
    for (id, (positive, negative)) in linked {
        let m = s
            .movements
            .iter()
            .find(|m| m.id == id)
            .expect("linked movement checked");
        let gross = nonnegative(m.trade_amount.as_deref().expect("trade validated"))?;
        let fee = nonnegative(&m.trade_fees)?;
        if (m.kind == "purchase" && (!positive.is_zero() || negative > &gross + &fee))
            || (m.kind == "sale" && (positive > gross || negative > fee))
        {
            return Err(invalid(
                "Les montants reliés au métal dépassent le prix ou les frais réellement déclarés.",
            ));
        }
    }
    Ok(())
}

fn warn(out: &mut Vec<MetalWarning>, code: &str, id: &str, message: &str) {
    if !out
        .iter()
        .any(|w| w.code == code && w.object_id == id && w.message == message)
    {
        out.push(MetalWarning {
            code: code.into(),
            object_id: id.into(),
            message: message.into(),
        });
    }
}
#[allow(clippy::too_many_arguments)] // Source, date and three independent trace collections are required for an exact historical conversion.
fn rate(
    registry: &RegistrySnapshot,
    from: &str,
    to: &str,
    date: &str,
    conversions: &mut Vec<FxResult>,
    sources: &mut BTreeSet<String>,
    warnings: &mut Vec<MetalWarning>,
    object: &str,
) -> Result<Option<BigRational>> {
    let report = convert_currency_unchecked(
        registry,
        &FxQuery {
            amount: "1".into(),
            from_currency: from.into(),
            to_currency: to.into(),
            date: date.into(),
            knowledge_sequence: Some(registry.knowledge_sequence),
        },
    )?;
    for w in &report.warnings {
        warn(warnings, &w.code, object, &w.message);
    }
    for leg in &report.legs {
        sources.extend([leg.rate_id.clone(), leg.revision_id.clone()]);
    }
    let value = report.exact_converted.as_ref().map(rational).transpose()?;
    conversions.push(report);
    Ok(value)
}
fn money(value: &Option<BigRational>, currency: &str) -> Result<Option<String>> {
    value
        .as_ref()
        .map(|v| format_currency(v, currency))
        .transpose()
}

struct CostHistory {
    native: Option<BigRational>,
    eur: Option<BigRational>,
    realized: Option<BigRational>,
    allocations: Vec<MetalCostAllocation>,
}
fn cost_history(
    registry: &RegistrySnapshot,
    s: &MetalSnapshot,
    lot: &MetalLot,
    date: &str,
    conversions: &mut Vec<FxResult>,
    sources: &mut BTreeSet<String>,
    warnings: &mut Vec<MetalWarning>,
) -> Result<CostHistory> {
    let mut quantity = BigRational::zero();
    let mut native = Some(BigRational::zero());
    let mut eur = Some(BigRational::zero());
    let mut realized = Some(BigRational::zero());
    let mut allocations = Vec::new();
    let mut opened = false;
    for m in movements(s, &lot.id, date) {
        sources.extend([m.id.clone(), m.revision_id.clone()]);
        if m.kind == "opening" {
            opened = true;
        }
        let change = parse_decimal(&m.quantity_change)?;
        if change.is_positive() {
            let cost = m.cost_amount.as_ref().map(|a| nonnegative(a)).transpose()?;
            let entry_eur = if let Some(c) = &cost {
                if c.is_zero() {
                    Some(BigRational::zero())
                } else {
                    rate(
                        registry,
                        &lot.cost_currency,
                        "EUR",
                        &m.effective_on,
                        conversions,
                        sources,
                        warnings,
                        &m.id,
                    )?
                    .map(|r| c * r)
                }
            } else {
                None
            };
            native = match (native, cost) {
                (Some(total), Some(c)) => Some(total + c),
                _ => None,
            };
            eur = match (eur, entry_eur) {
                (Some(total), Some(c)) => Some(total + c),
                _ => None,
            };
            quantity += change;
        } else if change.is_negative() {
            let outgoing = -change;
            if quantity.is_zero() {
                return Err(invalid("Coût de sortie impossible sans stock identifié."));
            }
            let proportion = &outgoing / &quantity;
            let removed_native = native.as_ref().map(|v| v * &proportion);
            let removed_eur = eur.as_ref().map(|v| v * &proportion);
            if let (Some(v), Some(c)) = (&mut native, &removed_native) {
                *v -= c;
            }
            if let (Some(v), Some(c)) = (&mut eur, &removed_eur) {
                *v -= c;
            }
            let proceeds = if m.kind == "sale" {
                let actual = metal_movement_cash_amount(m)?.expect("sale cash");
                if actual.is_zero() {
                    Some(BigRational::zero())
                } else {
                    rate(
                        registry,
                        m.trade_currency.as_deref().expect("sale currency checked"),
                        "EUR",
                        &m.effective_on,
                        conversions,
                        sources,
                        warnings,
                        &m.id,
                    )?
                    .map(|r| actual * r)
                }
            } else {
                None
            };
            let gain = match (&proceeds, &removed_eur) {
                (Some(p), Some(c)) => Some(p - c),
                _ => None,
            };
            if m.kind == "sale" {
                realized = match (realized, gain.clone()) {
                    (Some(total), Some(g)) => Some(total + g),
                    _ => None,
                };
            }
            allocations.push(MetalCostAllocation {
                movement_id: m.id.clone(),
                revision_id: m.revision_id.clone(),
                quantity: fraction_of(&outgoing),
                exact_cost_original: removed_native.as_ref().map(fraction_of),
                exact_cost_eur: removed_eur.as_ref().map(fraction_of),
                net_proceeds_eur: proceeds.as_ref().map(fraction_of),
                realized_gain_eur: gain.as_ref().map(fraction_of),
            });
            quantity -= outgoing;
            // Once the identified lot is entirely gone, its remaining cost is
            // exactly zero even when the removed historical basis was unknown.
            if quantity.is_zero() {
                native = Some(BigRational::zero());
                eur = Some(BigRational::zero());
            }
        } else if m.kind == "opening" && quantity.is_zero() {
            native = Some(BigRational::zero());
            eur = Some(BigRational::zero());
        }
    }
    if !opened {
        return Ok(CostHistory {
            native: None,
            eur: None,
            realized,
            allocations,
        });
    }
    if native.is_none() {
        warn(warnings,"missing_metal_cost",&lot.id,"Le prix de revient restant est inconnu : un coût d’entrée manque, sans empêcher la cotation du stock.");
    }
    Ok(CostHistory {
        native,
        eur,
        realized,
        allocations,
    })
}

pub fn metal_report(
    registry: &RegistrySnapshot,
    properties: &PropertySnapshot,
    treasury: &TreasurySnapshot,
    s: &MetalSnapshot,
    q: &MetalReportRequest,
) -> Result<MetalReport> {
    validate_metals(registry, properties, treasury, s)?;
    validate_date(&q.date)?;
    if q.knowledge_sequence
        .is_some_and(|k| k != s.knowledge_sequence)
    {
        return Err(invalid(
            "La borne de connaissance métal ne correspond pas aux sources.",
        ));
    }
    let lot = s
        .lots
        .iter()
        .find(|l| l.id == q.lot_id)
        .ok_or_else(|| invalid("Lot de métal inconnu."))?;
    let mut warnings = Vec::new();
    let mut sources = BTreeSet::from([
        lot.id.clone(),
        lot.revision_id.clone(),
        lot.asset_id.clone(),
        lot.legal_owner_entity_id.clone(),
    ]);
    let mut conversions = Vec::new();
    let quantity = metal_quantity_at(s, &lot.id, &q.date)?;
    let fine = quantity
        .as_ref()
        .map(|qty| {
            Ok(qty
                * metal_weight_in_grams(&lot.unit_weight, &lot.weight_unit)?
                * parse_decimal(&lot.purity)?)
        })
        .transpose()?;
    let CostHistory {
        native: cost_native,
        eur: cost_eur,
        realized,
        allocations,
    } = cost_history(
        registry,
        s,
        lot,
        &q.date,
        &mut conversions,
        &mut sources,
        &mut warnings,
    )?;
    if quantity.is_none() {
        warn(&mut warnings,"missing_metal_opening",&lot.id,"L’ouverture de quantité n’est pas confirmée à cette date ; un mouvement connu ne remplace pas le stock antérieur.");
    }
    let quote = s
        .quotes
        .iter()
        .filter(|r| r.metal == lot.metal && r.effective_on <= q.date)
        .max_by(|a, b| (&a.effective_on, &a.id).cmp(&(&b.effective_on, &b.id)))
        .cloned();
    let premium = s
        .premiums
        .iter()
        .filter(|r| r.lot_id == lot.id && r.effective_on <= q.date)
        .max_by(|a, b| (&a.effective_on, &a.id).cmp(&(&b.effective_on, &b.id)))
        .cloned();
    let zero_stock = quantity.as_ref().is_some_and(Zero::is_zero);
    // A confirmed empty lot has the same exact zero value in every currency.
    // Without a metal quote, report that valuation explicitly in EUR rather
    // than requiring a fictitious conversion from its historical cost currency.
    let currency = quote
        .as_ref()
        .map(|r| r.currency.clone())
        .unwrap_or_else(|| {
            if zero_stock {
                "EUR".into()
            } else {
                lot.cost_currency.clone()
            }
        });
    let intrinsic = if zero_stock {
        Some(BigRational::zero())
    } else if let (Some(weight), Some(price)) = (&fine, &quote) {
        Some(weight / metal_weight_in_grams("1", &price.weight_unit)? * parse_rate(&price.price)?)
    } else {
        None
    };
    if !zero_stock && quote.is_none() {
        warn(
            &mut warnings,
            "missing_metal_quote",
            &lot.id,
            "Aucun cours de métal antérieur ou égal à la date n’est disponible.",
        );
    }
    if let Some(price) = &quote {
        sources.extend([price.id.clone(), price.revision_id.clone()]);
        let day = |value: &str| -> time::Date {
            time::Date::from_calendar_date(
                value[..4].parse().expect("validated year"),
                time::Month::try_from(value[5..7].parse::<u8>().expect("validated month"))
                    .expect("validated month"),
                value[8..].parse().expect("validated day"),
            )
            .expect("validated date")
        };
        if (day(&q.date) - day(&price.effective_on)).whole_days() > 30 {
            warn(&mut warnings,"old_metal_quote",&price.id,"Le cours métal retenu date de plus de 30 jours ; sa date et sa source restent visibles.");
        }
        if s.quotes.iter().any(|other| {
            other.metal == price.metal
                && other.effective_on == price.effective_on
                && other.id != price.id
                && (other.price != price.price
                    || other.currency != price.currency
                    || other.weight_unit != price.weight_unit)
        }) {
            warn(&mut warnings,"conflicting_metal_quote",&price.id,"Plusieurs cours distincts existent pour ce métal à la même date ; le choix déterministe est tracé, sans moyenne implicite.");
        }
    }
    let rate_eur = if intrinsic.is_some() && !zero_stock {
        rate(
            registry,
            &currency,
            "EUR",
            &q.date,
            &mut conversions,
            &mut sources,
            &mut warnings,
            &lot.id,
        )?
    } else {
        Some(BigRational::one())
    };
    let intrinsic_eur = match (&intrinsic, &rate_eur) {
        (Some(v), Some(r)) => Some(v * r),
        _ => None,
    };
    let (premium_value, discount) = if zero_stock {
        (Some(BigRational::zero()), Some(BigRational::zero()))
    } else if let (Some(row), Some(qty)) = (&premium, &quantity) {
        sources.extend([row.id.clone(), row.revision_id.clone()]);
        let declared_premium = nonnegative(&row.value)?;
        let declared_discount = nonnegative(&row.discount_per_unit)?;
        let needs_conversion = (row.kind == "amount_per_unit" && !declared_premium.is_zero())
            || !declared_discount.is_zero();
        let conversion = if needs_conversion {
            rate(
                registry,
                &row.currency,
                &currency,
                &q.date,
                &mut conversions,
                &mut sources,
                &mut warnings,
                &row.id,
            )?
        } else {
            Some(BigRational::one())
        };
        let p = if declared_premium.is_zero() {
            Some(BigRational::zero())
        } else if row.kind == "fraction_of_intrinsic" {
            intrinsic.as_ref().map(|v| v * &declared_premium)
        } else {
            conversion.as_ref().map(|r| qty * &declared_premium * r)
        };
        let d = if declared_discount.is_zero() {
            Some(BigRational::zero())
        } else {
            conversion.as_ref().map(|r| qty * &declared_discount * r)
        };
        (p, d)
    } else {
        (None, None)
    };
    if !zero_stock && premium.is_none() {
        warn(&mut warnings,"missing_metal_resale_terms",&lot.id,"La prime et la décote de revente ne sont pas déclarées ; l’intrinsèque reste distinct d’une estimation de revente complète.");
    }
    let resale = match (&intrinsic, &premium_value, &discount) {
        (Some(i), Some(p), Some(d)) => {
            let value = i + p - d;
            if value.is_negative() {
                warn(&mut warnings,"negative_metal_resale",&lot.id,"La décote déclarée dépasse l’intrinsèque et la prime ; la valeur de revente reste non calculable.");
                None
            } else {
                Some(value)
            }
        }
        _ => None,
    };
    let premium_eur = match (&premium_value, &rate_eur) {
        (Some(v), Some(r)) => Some(v * r),
        _ => None,
    };
    let discount_eur = match (&discount, &rate_eur) {
        (Some(v), Some(r)) => Some(v * r),
        _ => None,
    };
    let resale_eur = match (&resale, &rate_eur) {
        (Some(v), Some(r)) => Some(v * r),
        _ => None,
    };
    let valuation = if lot.valuation_basis == "intrinsic" {
        intrinsic_eur.clone()
    } else {
        resale_eur.clone()
    };
    let unrealized = match (&valuation, &cost_eur) {
        (Some(v), Some(c)) => Some(v - c),
        _ => None,
    };
    let mut country = lot.country.clone();
    let mut storage = lot.storage_location.clone();
    for m in movements(s, &lot.id, &q.date) {
        if m.kind == "storage_transfer" {
            country = m.country.clone();
            storage = m.storage_location.clone();
        }
        if ["purchase", "sale"].contains(&m.kind.as_str()) && !metal_cash_coverage(s, m)? {
            warn(&mut warnings,"unlinked_metal_cash",&m.id,"Un achat ou une vente réelle n’est pas entièrement rapproché du journal ; aucun cash n’a été fabriqué.");
        }
        for link in s
            .cash_links
            .iter()
            .filter(|l| l.movement_id == m.id && l.movement_revision_id == m.revision_id)
        {
            sources.extend([
                link.id.clone(),
                link.revision_id.clone(),
                link.transaction_revision_id.clone(),
                link.line_id.clone(),
            ]);
        }
    }
    Ok(MetalReport {
        lot_id: lot.id.clone(),
        asset_id: lot.asset_id.clone(),
        date: q.date.clone(),
        knowledge_sequence: s.knowledge_sequence,
        status: if valuation.is_some() {
            "complete"
        } else {
            "partial"
        }
        .into(),
        valuation_basis: lot.valuation_basis.clone(),
        quantity_remaining: quantity.as_ref().map(physical_decimal),
        exact_quantity: quantity.as_ref().map(fraction_of),
        fine_weight_g: fine.as_ref().map(physical_decimal),
        exact_fine_weight_g: fine.as_ref().map(fraction_of),
        cost_currency: lot.cost_currency.clone(),
        currency: currency.clone(),
        cost_remaining_original: money(&cost_native, &lot.cost_currency)?,
        exact_cost_remaining_original: cost_native.as_ref().map(fraction_of),
        cost_remaining_eur: cost_eur.as_ref().map(format_eur),
        exact_cost_remaining_eur: cost_eur.as_ref().map(fraction_of),
        quote,
        premium,
        intrinsic_original: money(&intrinsic, &currency)?,
        exact_intrinsic_original: intrinsic.as_ref().map(fraction_of),
        intrinsic_eur: intrinsic_eur.as_ref().map(format_eur),
        exact_intrinsic_eur: intrinsic_eur.as_ref().map(fraction_of),
        premium_original: money(&premium_value, &currency)?,
        exact_premium_original: premium_value.as_ref().map(fraction_of),
        premium_eur: premium_eur.as_ref().map(format_eur),
        exact_premium_eur: premium_eur.as_ref().map(fraction_of),
        discount_original: money(&discount, &currency)?,
        exact_discount_original: discount.as_ref().map(fraction_of),
        discount_eur: discount_eur.as_ref().map(format_eur),
        exact_discount_eur: discount_eur.as_ref().map(fraction_of),
        resale_original: money(&resale, &currency)?,
        exact_resale_original: resale.as_ref().map(fraction_of),
        resale_eur: resale_eur.as_ref().map(format_eur),
        exact_resale_eur: resale_eur.as_ref().map(fraction_of),
        valuation_eur: valuation.as_ref().map(format_eur),
        exact_valuation_eur: valuation.as_ref().map(fraction_of),
        realized_gain_eur: realized.as_ref().map(format_eur),
        exact_realized_gain_eur: realized.as_ref().map(fraction_of),
        unrealized_gain_eur: unrealized.as_ref().map(format_eur),
        exact_unrealized_gain_eur: unrealized.as_ref().map(fraction_of),
        country,
        storage_location: storage,
        cost_allocations: allocations,
        conversions,
        source_ids: sources.into_iter().collect(),
        warnings,
    })
}
