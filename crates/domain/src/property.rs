//! Property records are descriptive extensions of one existing asset. Rental
//! units are technical subdivisions and never create another valued asset.
use crate::exact::{format_currency, format_eur, fraction_of, parse_decimal};
use crate::fx::{convert_currency_unchecked, converted_rational};
use crate::*;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

macro_rules! property_record {
    ($row:ident, $input:ident, { $( $field:ident: $ty:ty ),* $(,)? }) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row { pub id: String, pub revision_id: String, $(pub $field: $ty,)* }
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input { pub id: Option<String>, pub expected_revision_id: Option<String>, pub reason: String, $(pub $field: $ty,)* }
    };
}

property_record!(Property, SavePropertyInput, {
    asset_id: String,
    property_type: String,
    city: String,
    address: String,
    surface_m2: Option<String>,
    status: String,
    valid_from: String,
    valid_to: Option<String>,
    notes: String,
});
property_record!(RentalUnit, SaveRentalUnitInput, {
    property_asset_id: String,
    reference: String,
    name: String,
    surface_m2: Option<String>,
    valid_from: String,
    valid_to: Option<String>,
    available_for_rent: bool,
    notes: String,
});
property_record!(PropertyEvent, SavePropertyEventInput, {
    asset_id: String,
    kind: String,
    effective_on: String,
    price: Option<String>,
    fees: Option<String>,
    currency: String,
    notes: String,
});

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertySnapshot {
    pub properties: Vec<Property>,
    pub units: Vec<RentalUnit>,
    pub events: Vec<PropertyEvent>,
    pub knowledge_sequence: u64,
}

/// Full disposal is an explicit atomic command: close the tracked ownership
/// intervals and append the disposal event under one change set. It neither
/// fabricates cash nor deletes the asset or its historical estimates.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisposePropertyInput {
    pub asset_id: String,
    pub effective_on: String,
    pub price: Option<String>,
    pub fees: Option<String>,
    pub currency: String,
    pub reason: String,
    pub expected_sequence: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PropertyReportRequest {
    pub asset_id: String,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyReport {
    pub asset_id: String,
    pub label: String,
    pub country: Option<String>,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub property: Option<Property>,
    pub units: Vec<RentalUnit>,
    pub acquisition: Option<PropertyEvent>,
    pub disposal: Option<PropertyEvent>,
    pub acquisition_cost: Option<String>,
    pub exact_acquisition_cost: Option<Fraction>,
    pub acquisition_currency: Option<String>,
    pub acquisition_cost_eur: Option<String>,
    pub exact_acquisition_cost_eur: Option<Fraction>,
    pub valuation: Option<Valuation>,
    pub valuation_conversion: Option<FxResult>,
    pub acquisition_conversions: Vec<FxResult>,
    pub estimated_value_eur: Option<String>,
    pub exact_estimated_value_eur: Option<Fraction>,
    pub unrealized_gain_original: Option<String>,
    pub exact_unrealized_gain_original: Option<Fraction>,
    pub unrealized_gain_currency: Option<String>,
    pub unrealized_gain_eur: Option<String>,
    pub exact_unrealized_gain_eur: Option<Fraction>,
    pub debt_remaining_eur: Option<String>,
    pub net_before_disposal_eur: Option<String>,
    #[serde(default)]
    pub exact_debt_remaining_eur: Option<Fraction>,
    #[serde(default)]
    pub exact_net_before_disposal_eur: Option<Fraction>,
    #[serde(default)]
    pub financing_conversions: Vec<FxResult>,
    pub source_ids: Vec<String>,
    pub warnings: Vec<PropertyWarning>,
}

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
        return Err(invalid("Un identifiant immobilier est invalide."));
    }
    Ok(())
}
fn text(value: &str, length: usize, required: bool) -> Result<()> {
    if value.chars().count() > length
        || (required && value.trim().is_empty())
        || value
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(invalid("Une désignation immobilière est vide, trop longue ou contient des caractères interdits."));
    }
    Ok(())
}
fn positive_optional(value: &Option<String>, allow_zero: bool) -> Result<()> {
    if let Some(value) = value {
        let amount = parse_decimal(value)?;
        if amount.is_negative() || (!allow_zero && amount.is_zero()) {
            return Err(invalid("Une surface doit être strictement positive ; prix et frais doivent être positifs ou nuls."));
        }
    }
    Ok(())
}

/// Both snapshots must be read in the same SQLite transaction at the same K.
/// This validates all recorded civil dates, including future intervals.
pub fn validate_properties(registry: &RegistrySnapshot, snapshot: &PropertySnapshot) -> Result<()> {
    validate_registry(registry)?;
    if registry.knowledge_sequence != snapshot.knowledge_sequence {
        return Err(invalid(
            "Les données immobilières et le registre doivent avoir la même borne de connaissance.",
        ));
    }
    let mut ids: HashSet<&str> = HashSet::new();
    let mut revisions: HashSet<&str> = HashSet::new();
    let registry_keys: HashSet<_> = registry
        .entities
        .iter()
        .map(|r| (&r.id, &r.revision_id))
        .chain(registry.assets.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(registry.memberships.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(registry.ownerships.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(
            registry
                .participations
                .iter()
                .map(|r| (&r.id, &r.revision_id)),
        )
        .chain(registry.valuations.iter().map(|r| (&r.id, &r.revision_id)))
        .chain(
            registry
                .company_policies
                .iter()
                .map(|r| (&r.id, &r.revision_id)),
        )
        .chain(
            registry
                .exchange_rates
                .iter()
                .map(|r| (&r.id, &r.revision_id)),
        )
        .chain(
            registry
                .profile_periods
                .iter()
                .map(|r| (&r.id, &r.revision_id)),
        )
        .collect();
    for (id, revision) in snapshot
        .properties
        .iter()
        .map(|p| (&p.id, &p.revision_id))
        .chain(snapshot.units.iter().map(|p| (&p.id, &p.revision_id)))
        .chain(snapshot.events.iter().map(|p| (&p.id, &p.revision_id)))
    {
        identifier(id)?;
        identifier(revision)?;
        if !ids.insert(id)
            || !revisions.insert(revision)
            || registry_keys
                .iter()
                .any(|(rid, rev)| *rid == id || *rev == revision)
        {
            return Err(invalid(
                "Une identité ou révision immobilière apparaît plusieurs fois.",
            ));
        }
    }
    let asset_exists = |id: &str| registry.assets.iter().any(|a| a.id == id);
    let mut periods: BTreeMap<&str, Vec<(&str, Option<&str>)>> = BTreeMap::new();
    for property in &snapshot.properties {
        if !asset_exists(&property.asset_id) {
            return Err(invalid(
                "Le bien doit compléter un actif existant du registre.",
            ));
        }
        if ![
            "apartment",
            "house",
            "building",
            "land",
            "commercial",
            "office",
            "garage",
            "other",
        ]
        .contains(&property.property_type.as_str())
        {
            return Err(invalid("Le type immobilier est inconnu."));
        }
        if ![
            "owner_occupied",
            "rented",
            "vacant",
            "renovation",
            "for_sale",
        ]
        .contains(&property.status.as_str())
        {
            return Err(invalid("Le statut vendu résulte d’une cession atomique, pas d’un simple changement de fiche."));
        }
        validate_interval(&property.valid_from, property.valid_to.as_deref())?;
        text(&property.city, 160, false)?;
        text(&property.address, 1000, false)?;
        text(&property.notes, 10000, false)?;
        positive_optional(&property.surface_m2, false)?;
        periods
            .entry(&property.asset_id)
            .or_default()
            .push((&property.valid_from, property.valid_to.as_deref()));
    }
    for rows in periods.values_mut() {
        rows.sort();
        if rows
            .windows(2)
            .any(|p| p[0].1.is_none_or(|end| p[1].0 < end))
        {
            return Err(invalid(
                "Deux fiches descriptives du même bien se chevauchent.",
            ));
        }
    }
    let is_property = |id: &str| snapshot.properties.iter().any(|p| p.asset_id == id);
    let mut unit_references = HashSet::new();
    for unit in &snapshot.units {
        if !is_property(&unit.property_asset_id) {
            return Err(invalid("L’unité doit appartenir à un bien identifié."));
        }
        text(&unit.reference, 80, true)?;
        text(&unit.name, 160, true)?;
        text(&unit.notes, 10000, false)?;
        positive_optional(&unit.surface_m2, false)?;
        validate_interval(&unit.valid_from, unit.valid_to.as_deref())?;
        if !unit_references.insert((&unit.property_asset_id, &unit.reference)) {
            return Err(invalid(
                "La référence de l’unité est déjà utilisée dans ce bien.",
            ));
        }
    }
    let mut events: BTreeMap<&str, Vec<&PropertyEvent>> = BTreeMap::new();
    for event in &snapshot.events {
        if !is_property(&event.asset_id) {
            return Err(invalid("Le cycle de vie doit concerner un bien identifié."));
        }
        if !["acquisition", "opening_tracking", "disposal"].contains(&event.kind.as_str()) {
            return Err(invalid("Le type de fait immobilier est inconnu."));
        }
        validate_date(&event.effective_on)?;
        crate::exact::currency_minor_units(&event.currency)?;
        positive_optional(&event.price, true)?;
        positive_optional(&event.fees, true)?;
        if event.kind == "opening_tracking" && (event.price.is_some() || event.fees.is_some()) {
            return Err(invalid("Une ouverture de suivi n’est pas une acquisition connue. Saisissez une acquisition à sa vraie date pour renseigner son prix et ses frais."));
        }
        if event.kind == "acquisition"
            && registry
                .ownerships
                .iter()
                .any(|o| o.asset_id == event.asset_id && o.valid_from < event.effective_on)
        {
            return Err(invalid("Une acquisition connue ne peut pas être postérieure à un droit de propriété déjà enregistré pour ce bien."));
        }
        if event.kind == "disposal"
            && registry.ownerships.iter().any(|o| {
                o.asset_id == event.asset_id
                    && o.valid_to
                        .as_ref()
                        .is_none_or(|end| end > &event.effective_on)
            })
        {
            return Err(invalid("La cession exige la clôture atomique des droits à cette date ; un droit actif ou futur subsiste après la cession."));
        }
        text(&event.notes, 10000, false)?;
        events.entry(&event.asset_id).or_default().push(event);
    }
    for rows in events.values_mut() {
        rows.sort_by(|a, b| a.effective_on.cmp(&b.effective_on));
        let entries: Vec<_> = rows.iter().filter(|e| e.kind != "disposal").collect();
        let disposals: Vec<_> = rows.iter().filter(|e| e.kind == "disposal").collect();
        if entries.len() > 1
            || disposals.len() > 1
            || (!disposals.is_empty() && entries.is_empty())
            || entries
                .first()
                .zip(disposals.first())
                .is_some_and(|(entry, disposal)| disposal.effective_on <= entry.effective_on)
        {
            return Err(invalid("Le cycle du bien doit avoir une seule acquisition ou ouverture, suivie au plus d’une cession strictement postérieure."));
        }
    }
    Ok(())
}

fn warn(warnings: &mut Vec<PropertyWarning>, code: &str, message: &str) {
    warnings.push(PropertyWarning {
        code: code.into(),
        message: message.into(),
    });
}

fn convert(
    registry: &RegistrySnapshot,
    amount: &str,
    currency: &str,
    date: &str,
) -> Result<FxResult> {
    convert_currency_unchecked(
        registry,
        &FxQuery {
            amount: amount.into(),
            from_currency: currency.into(),
            to_currency: "EUR".into(),
            date: date.into(),
            knowledge_sequence: Some(registry.knowledge_sequence),
        },
    )
}

fn age_days(from: &str, to: &str) -> i64 {
    let date = |value: &str| {
        time::Date::from_calendar_date(
            value[..4].parse().expect("validated year"),
            time::Month::try_from(value[5..7].parse::<u8>().expect("validated month"))
                .expect("validated month"),
            value[8..].parse().expect("validated day"),
        )
        .expect("validated date")
    };
    (date(to) - date(from)).whole_days()
}

/// A descriptive report on the whole property, independent from the share
/// attributed to one person. Disposal removes its current stock estimate from
/// this report while preserving its cost, sources and historical estimates.
pub fn property_report(
    registry: &RegistrySnapshot,
    snapshot: &PropertySnapshot,
    query: &PropertyReportRequest,
) -> Result<PropertyReport> {
    validate_properties(registry, snapshot)?;
    validate_date(&query.date)?;
    if query
        .knowledge_sequence
        .is_some_and(|k| k != snapshot.knowledge_sequence)
    {
        return Err(invalid(
            "La borne de connaissance ne correspond pas aux données du rapport.",
        ));
    }
    let overlay = apply_profile_periods(registry, &query.date)?;
    let asset = overlay
        .assets
        .iter()
        .find(|a| a.id == query.asset_id)
        .ok_or_else(|| invalid("Cet actif immobilier est inconnu."))?;
    if !snapshot
        .properties
        .iter()
        .any(|p| p.asset_id == query.asset_id)
    {
        return Err(invalid("Cet actif n’a pas de fiche immobilière."));
    }
    let property = snapshot
        .properties
        .iter()
        .find(|p| {
            p.asset_id == query.asset_id
                && applies(&p.valid_from, p.valid_to.as_deref(), &query.date)
        })
        .cloned();
    let acquisition = snapshot
        .events
        .iter()
        .find(|e| {
            e.asset_id == query.asset_id && e.kind != "disposal" && e.effective_on <= query.date
        })
        .cloned();
    let disposal = snapshot
        .events
        .iter()
        .find(|e| {
            e.asset_id == query.asset_id && e.kind == "disposal" && e.effective_on <= query.date
        })
        .cloned();
    let units = snapshot
        .units
        .iter()
        .filter(|u| {
            u.property_asset_id == query.asset_id
                && applies(&u.valid_from, u.valid_to.as_deref(), &query.date)
        })
        .cloned()
        .collect();
    let valuation = registry
        .valuations
        .iter()
        .filter(|v| {
            v.target_kind == "asset"
                && v.target_id == query.asset_id
                && v.effective_on <= query.date
        })
        .max_by(|a, b| a.effective_on.cmp(&b.effective_on))
        .cloned();
    let profile_dated = registry.profile_periods.iter().any(|p| {
        p.target_kind == "asset"
            && p.target_id == asset.id
            && applies(&p.valid_from, p.valid_to.as_deref(), &query.date)
    });
    let mut out = PropertyReport {
        asset_id: asset.id.clone(),
        label: asset.name.clone(),
        country: profile_dated.then(|| asset.country.clone()),
        date: query.date.clone(),
        knowledge_sequence: snapshot.knowledge_sequence,
        status: if disposal.is_some() {
            "sold".into()
        } else {
            property
                .as_ref()
                .map(|p| p.status.clone())
                .unwrap_or_else(|| "not_recorded".into())
        },
        property,
        units,
        acquisition: acquisition.clone(),
        disposal,
        acquisition_cost: None,
        exact_acquisition_cost: None,
        acquisition_currency: acquisition.as_ref().map(|e| e.currency.clone()),
        acquisition_cost_eur: None,
        exact_acquisition_cost_eur: None,
        valuation: valuation.clone(),
        valuation_conversion: None,
        acquisition_conversions: Vec::new(),
        estimated_value_eur: None,
        exact_estimated_value_eur: None,
        unrealized_gain_original: None,
        exact_unrealized_gain_original: None,
        unrealized_gain_currency: None,
        unrealized_gain_eur: None,
        exact_unrealized_gain_eur: None,
        debt_remaining_eur: None,
        net_before_disposal_eur: None,
        exact_debt_remaining_eur: None,
        exact_net_before_disposal_eur: None,
        financing_conversions: Vec::new(),
        source_ids: vec![asset.id.clone(), asset.revision_id.clone()],
        warnings: Vec::new(),
    };
    if let Some(p) = &out.property {
        out.source_ids.extend([p.id.clone(), p.revision_id.clone()]);
    }
    for unit in &out.units {
        out.source_ids
            .extend([unit.id.clone(), unit.revision_id.clone()]);
    }
    for period in registry.profile_periods.iter().filter(|p| {
        p.target_kind == "asset"
            && p.target_id == asset.id
            && applies(&p.valid_from, p.valid_to.as_deref(), &query.date)
    }) {
        out.source_ids.extend([
            period.id.clone(),
            period.revision_id.clone(),
            period.profile_revision_id.clone(),
        ]);
    }
    if !profile_dated {
        warn(&mut out.warnings, "undated_profile", "Le libellé reflète les connaissances sélectionnées. Sans période descriptive de l’actif à cette date, son pays historique reste inconnu.");
    }
    if let Some(event) = &acquisition {
        out.source_ids
            .extend([event.id.clone(), event.revision_id.clone()]);
    }
    if let Some(event) = &out.disposal {
        out.source_ids
            .extend([event.id.clone(), event.revision_id.clone()]);
    }
    if let Some(value) = &valuation {
        out.source_ids
            .extend([value.id.clone(), value.revision_id.clone()]);
    }
    if out.property.is_none() {
        warn(
            &mut out.warnings,
            "missing_property_period",
            "Aucune fiche descriptive immobilière ne s’applique à cette date.",
        );
    }
    let cost = acquisition
        .as_ref()
        .and_then(|e| e.price.as_ref().zip(e.fees.as_ref()))
        .map(|(p, f)| Ok(parse_decimal(p)? + parse_decimal(f)?))
        .transpose()?;
    if let (Some(event), Some(cost)) = (&acquisition, &cost) {
        out.acquisition_cost = Some(format_currency(cost, &event.currency)?);
        out.exact_acquisition_cost = Some(fraction_of(cost));
        let mut cost_eur = BigRational::zero();
        let mut complete = true;
        for amount in [&event.price, &event.fees].into_iter().flatten() {
            let conversion = convert(registry, amount, &event.currency, &event.effective_on)?;
            if let Some(value) = converted_rational(&conversion) {
                cost_eur += value;
            } else {
                complete = false;
            }
            for w in &conversion.warnings {
                warn(&mut out.warnings, &w.code, &w.message);
            }
            for leg in &conversion.legs {
                out.source_ids
                    .extend([leg.rate_id.clone(), leg.revision_id.clone()]);
            }
            out.acquisition_conversions.push(conversion);
        }
        if complete {
            out.acquisition_cost_eur = Some(format_eur(&cost_eur));
            out.exact_acquisition_cost_eur = Some(fraction_of(&cost_eur));
        }
    } else {
        warn(&mut out.warnings, "unknown_acquisition_cost", "Prix et frais d’acquisition doivent tous deux être connus, y compris des frais explicitement nuls. Le coût n’est pas une estimation actuelle par défaut.");
    }
    if out.disposal.is_some() {
        warn(&mut out.warnings, "property_disposed", "Le bien est cédé à cette date. Les anciennes estimations restent consultables, mais ne constituent plus un stock détenu ni une plus-value latente. Aucun produit de vente en trésorerie n’est inventé.");
    } else if let Some(value) = &valuation {
        if age_days(&value.effective_on, &query.date) > 365 {
            warn(&mut out.warnings, "stale_property_valuation", "L’estimation immobilière a plus de 365 jours ; son montant reste calculé avec sa date et sa source, sans mise à jour supposée.");
        }
        let conversion = convert(registry, &value.amount, &value.currency, &query.date)?;
        for w in &conversion.warnings {
            warn(&mut out.warnings, &w.code, &w.message);
        }
        for leg in &conversion.legs {
            out.source_ids
                .extend([leg.rate_id.clone(), leg.revision_id.clone()]);
        }
        let exact_value = converted_rational(&conversion);
        out.estimated_value_eur = exact_value.as_ref().map(format_eur);
        out.exact_estimated_value_eur = exact_value.as_ref().map(fraction_of);
        out.valuation_conversion = Some(conversion);
        if let Some(event) = &acquisition {
            if let Some(cost) = &cost {
                if value.currency == event.currency {
                    let gain = parse_decimal(&value.amount)? - cost;
                    out.unrealized_gain_original = Some(format_currency(&gain, &value.currency)?);
                    out.exact_unrealized_gain_original = Some(fraction_of(&gain));
                    out.unrealized_gain_currency = Some(value.currency.clone());
                }
            }
        }
        if let (Some(value), Some(cost)) = (exact_value, out.exact_acquisition_cost_eur.as_ref()) {
            let cost = BigRational::new(
                cost.num
                    .parse()
                    .map_err(|_| invalid("Coût exact invalide."))?,
                cost.den
                    .parse()
                    .map_err(|_| invalid("Coût exact invalide."))?,
            );
            let gain = value - cost;
            out.unrealized_gain_eur = Some(format_eur(&gain));
            out.exact_unrealized_gain_eur = Some(fraction_of(&gain));
        }
    } else {
        warn(&mut out.warnings, "missing_valuation", "Aucune estimation applicable n’est connue. Le prix d’achat n’est pas utilisé comme valeur actuelle.");
    }
    warn(&mut out.warnings, "financing_not_inventoried", "Les financements ne sont pas encore inventoriés. La dette restante et la valeur nette avant cession sont inconnues, jamais égales à zéro par hypothèse.");
    out.source_ids.sort();
    out.source_ids.dedup();
    Ok(out)
}
