use crate::{exact::*, *};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};
use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

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
        return Err(invalid("Un identifiant du registre est invalide."));
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
            "Une désignation est vide, trop longue ou contient des caractères interdits.",
        ));
    }
    Ok(())
}
fn code(value: &str, size: usize) -> Result<()> {
    if value.len() != size || !value.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(invalid("Un code de pays ou de devise est invalide."));
    }
    Ok(())
}
fn rights(values: &[&Option<Fraction>]) -> Result<()> {
    for value in values.iter().filter_map(|v| v.as_ref()) {
        parse_fraction(value)?;
    }
    Ok(())
}

fn nonoverlapping(mut rows: Vec<(String, &str, Option<&str>)>) -> Result<()> {
    rows.sort_by(|a, b| (&a.0, a.1).cmp(&(&b.0, b.1)));
    for pair in rows.windows(2) {
        if pair[0].0 == pair[1].0 && pair[0].2.is_none_or(|end| pair[1].1 < end) {
            return Err(invalid(
                "Deux périodes du même droit, du même membre ou de la même méthode se chevauchent.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn company_order(s: &RegistrySnapshot, date: &str) -> Result<Vec<String>> {
    let companies: BTreeSet<_> = s
        .entities
        .iter()
        .filter(|e| e.kind == "company")
        .map(|e| e.id.clone())
        .collect();
    let mut indegrees: BTreeMap<_, usize> = companies.iter().map(|id| (id.clone(), 0)).collect();
    let mut outgoing: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for p in s
        .participations
        .iter()
        .filter(|p| applies(&p.valid_from, p.valid_to.as_deref(), date))
    {
        if companies.contains(&p.owner_entity_id) {
            *indegrees
                .get_mut(&p.company_id)
                .ok_or_else(|| invalid("La société détenue est inconnue."))? += 1;
            outgoing
                .entry(p.owner_entity_id.clone())
                .or_default()
                .push(p.company_id.clone());
        }
    }
    let mut queue: VecDeque<_> = indegrees
        .iter()
        .filter(|(_, n)| **n == 0)
        .map(|(id, _)| id.clone())
        .collect();
    let mut order = Vec::with_capacity(companies.len());
    while let Some(id) = queue.pop_front() {
        for child in outgoing.get(&id).into_iter().flatten() {
            let n = indegrees.get_mut(child).expect("validated company");
            *n -= 1;
            if *n == 0 {
                queue.push_back(child.clone());
            }
        }
        order.push(id);
    }
    if order.len() != companies.len() {
        return Err(invalid(&format!(
            "Un cycle de détention est actif au {date}. La modification entière est refusée."
        )));
    }
    Ok(order)
}

/// Structural and economic invariants for every date affected by the registry.
/// Unknown rights are allowed but are never made into zero or full ownership.
pub fn validate_registry(s: &RegistrySnapshot) -> Result<()> {
    let mut identities = HashSet::new();
    let mut revisions = HashSet::new();
    let records: Vec<_> = s
        .entities
        .iter()
        .map(|v| (&v.id, &v.revision_id))
        .chain(s.memberships.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.participations.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.assets.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.ownerships.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.valuations.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.company_policies.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.exchange_rates.iter().map(|v| (&v.id, &v.revision_id)))
        .chain(s.profile_periods.iter().map(|v| (&v.id, &v.revision_id)))
        .collect();
    for (id, revision) in records {
        identifier(id)?;
        identifier(revision)?;
        if !identities.insert(id) || !revisions.insert(revision) {
            return Err(invalid(
                "Un objet ou une révision apparaît plusieurs fois dans le registre.",
            ));
        }
    }
    let entities: BTreeMap<_, _> = s.entities.iter().map(|e| (e.id.as_str(), e)).collect();
    let assets: BTreeSet<_> = s.assets.iter().map(|a| a.id.as_str()).collect();
    let is_person = |id: &str| entities.get(id).is_some_and(|e| e.kind == "person");
    let is_company = |id: &str| entities.get(id).is_some_and(|e| e.kind == "company");
    let mut historical_revisions = HashSet::new();
    let mut historical_kinds = BTreeMap::new();
    for e in &s.profile_revisions {
        identifier(&e.id)?;
        identifier(&e.revision_id)?;
        let current = entities.get(e.id.as_str());
        // A void removes the current profile, never its recorded history.
        // Historical identities must still keep their original object kind.
        if (identities.contains(&e.id) && current.is_none())
            || current.is_some_and(|c| e.kind != c.kind)
            || historical_kinds
                .insert(&e.id, e.kind.as_str())
                .is_some_and(|kind| kind != e.kind)
            || !historical_revisions.insert(&e.revision_id)
            || (revisions.contains(&e.revision_id)
                && !current.is_some_and(|c| c.revision_id == e.revision_id && *c == e))
        {
            return Err(invalid(
                "Une révision historique de profil est ambiguë ou change la nature de l’entité.",
            ));
        }
    }
    for a in &s.asset_profile_revisions {
        identifier(&a.id)?;
        identifier(&a.revision_id)?;
        let current = s.assets.iter().find(|current| current.id == a.id);
        if (identities.contains(&a.id) && current.is_none())
            || historical_kinds
                .insert(&a.id, "asset")
                .is_some_and(|kind| kind != "asset")
            || !historical_revisions.insert(&a.revision_id)
            || (revisions.contains(&a.revision_id)
                && !current.is_some_and(|c| c.revision_id == a.revision_id && c == a))
        {
            return Err(invalid("Une révision historique de profil est ambiguë."));
        }
    }
    for e in s.entities.iter().chain(&s.profile_revisions) {
        if !["person", "company"].contains(&e.kind.as_str()) {
            return Err(invalid(
                "La nature de l’entité doit être personne ou société.",
            ));
        }
        text(&e.display_name, 160, true)?;
        for name in [&e.first_name, &e.last_name].into_iter().flatten() {
            text(name, 100, false)?;
        }
        if let Some(date) = &e.birth_date {
            validate_date(date)?;
            if e.kind != "person" {
                return Err(invalid(
                    "Une date de naissance ne concerne qu’une personne.",
                ));
            }
        }
        if let Some(country) = &e.residence_country {
            code(country, 2)?;
        }
        code(&e.preferred_currency, 3)?;
        text(&e.activity, 1000, false)?;
        text(&e.notes, 10000, false)?;
    }
    for a in s.assets.iter().chain(&s.asset_profile_revisions) {
        text(&a.name, 160, true)?;
        code(&a.country, 2)?;
        text(&a.category, 80, true)?;
        text(&a.notes, 10000, false)?;
    }
    let mut boundaries = BTreeSet::new();
    boundaries.insert("0001-01-01".to_owned());
    let mut interval = |from: &str, to: Option<&str>| -> Result<()> {
        validate_interval(from, to)?;
        boundaries.insert(from.to_owned());
        if let Some(to) = to {
            boundaries.insert(to.to_owned());
        }
        Ok(())
    };
    for m in &s.memberships {
        if !is_person(&m.person_id) {
            return Err(invalid(
                "Le membre du périmètre doit être une personne identifiée.",
            ));
        }
        interval(&m.valid_from, m.valid_to.as_deref())?;
        text(&m.relationship, 160, false)?;
    }
    for p in &s.participations {
        if !entities.contains_key(p.owner_entity_id.as_str())
            || !is_company(&p.company_id)
            || p.owner_entity_id == p.company_id
        {
            return Err(invalid(
                "Une participation exige un détenteur connu et une société distincte.",
            ));
        }
        interval(&p.valid_from, p.valid_to.as_deref())?;
        rights(&[&p.legal_share, &p.economic_share, &p.voting_share])?;
    }
    for o in &s.ownerships {
        if !entities.contains_key(o.owner_entity_id.as_str())
            || !assets.contains(o.asset_id.as_str())
        {
            return Err(invalid("Le propriétaire ou l’actif du droit est inconnu."));
        }
        interval(&o.valid_from, o.valid_to.as_deref())?;
        rights(&[&o.legal_share, &o.economic_share])?;
    }
    for p in &s.company_policies {
        if !is_company(&p.company_id) || !["detailed", "global_equity"].contains(&p.mode.as_str()) {
            return Err(invalid(
                "La méthode de valorisation d’une société est invalide.",
            ));
        }
        interval(&p.valid_from, p.valid_to.as_deref())?;
    }
    for p in &s.profile_periods {
        identifier(&p.profile_revision_id)?;
        let exists = match p.target_kind.as_str() {
            "entity" => {
                entities.contains_key(p.target_id.as_str())
                    && s.entities
                        .iter()
                        .chain(&s.profile_revisions)
                        .any(|e| e.id == p.target_id && e.revision_id == p.profile_revision_id)
            }
            "asset" => {
                assets.contains(p.target_id.as_str())
                    && s.assets
                        .iter()
                        .chain(&s.asset_profile_revisions)
                        .any(|a| a.id == p.target_id && a.revision_id == p.profile_revision_id)
            }
            _ => false,
        };
        if !exists {
            return Err(invalid(
                "La période doit référencer une révision exacte de son propre profil.",
            ));
        }
        validate_interval(&p.valid_from, p.valid_to.as_deref())?;
    }
    nonoverlapping(
        s.profile_periods
            .iter()
            .map(|p| {
                (
                    format!("{}|{}", p.target_kind, p.target_id),
                    p.valid_from.as_str(),
                    p.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    nonoverlapping(
        s.memberships
            .iter()
            .map(|m| {
                (
                    m.person_id.clone(),
                    m.valid_from.as_str(),
                    m.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    nonoverlapping(
        s.company_policies
            .iter()
            .map(|p| {
                (
                    p.company_id.clone(),
                    p.valid_from.as_str(),
                    p.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    nonoverlapping(
        s.participations
            .iter()
            .map(|p| {
                (
                    format!("{}|{}", p.owner_entity_id, p.company_id),
                    p.valid_from.as_str(),
                    p.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    nonoverlapping(
        s.ownerships
            .iter()
            .map(|o| {
                (
                    format!("{}|{}", o.owner_entity_id, o.asset_id),
                    o.valid_from.as_str(),
                    o.valid_to.as_deref(),
                )
            })
            .collect(),
    )?;
    let mut valuation_keys = HashSet::new();
    for v in &s.valuations {
        validate_date(&v.effective_on)?;
        code(&v.currency, 3)?;
        if v.source.trim().is_empty() {
            return Err(invalid("Indiquez la source ou la méthode de cette estimation, par exemple une estimation personnelle."));
        }
        text(&v.source, 1000, true)?;
        let amount = parse_decimal(&v.amount)?;
        match v.target_kind.as_str() {
            "asset" if assets.contains(v.target_id.as_str()) && !amount.is_negative() => {}
            "company" if is_company(&v.target_id) => {}
            _ => {
                return Err(invalid(
                    "La cible ou le signe de la valorisation est invalide.",
                ))
            }
        }
        if !valuation_keys.insert((&v.target_kind, &v.target_id, &v.effective_on)) {
            return Err(invalid("Deux estimations concurrentes existent à la même date. Corrigez la révision existante."));
        }
    }
    let mut rate_keys = HashSet::new();
    for rate in &s.exchange_rates {
        code(&rate.base_currency, 3)?;
        code(&rate.quote_currency, 3)?;
        if rate.base_currency == rate.quote_currency {
            return Err(invalid("Un taux manuel doit relier deux devises distinctes. L’identité vaut exactement un."));
        }
        parse_rate(&rate.rate)?;
        validate_date(&rate.effective_on)?;
        if let Some(source) = &rate.source {
            text(source, 1000, true)?;
        }
        text(&rate.notes, 10000, false)?;
        if !rate_keys.insert((
            &rate.base_currency,
            &rate.quote_currency,
            &rate.effective_on,
        )) {
            return Err(invalid("Deux taux concurrents existent pour la même paire et date. Corrigez la révision existante."));
        }
    }
    let edge_count = s
        .participations
        .len()
        .saturating_add(s.ownerships.len())
        .max(1);
    if boundaries.len().saturating_mul(edge_count) > 5_000_000 {
        return Err(AppError::new("capacity","La vérification temporelle dépasse le budget de calcul du socle. Aucune écriture n’a été effectuée."));
    }
    for date in boundaries {
        company_order(s, &date)?;
        let mut sums: BTreeMap<(String, usize), BigRational> = BTreeMap::new();
        for p in s
            .participations
            .iter()
            .filter(|p| applies(&p.valid_from, p.valid_to.as_deref(), &date))
        {
            for (i, value) in [&p.legal_share, &p.economic_share, &p.voting_share]
                .into_iter()
                .enumerate()
            {
                if let Some(value) = value {
                    *sums
                        .entry((format!("company:{}", p.company_id), i))
                        .or_insert_with(BigRational::zero) += parse_fraction(value)?;
                }
            }
        }
        for o in s
            .ownerships
            .iter()
            .filter(|o| applies(&o.valid_from, o.valid_to.as_deref(), &date))
        {
            for (i, value) in [&o.legal_share, &o.economic_share].into_iter().enumerate() {
                if let Some(value) = value {
                    *sums
                        .entry((format!("asset:{}", o.asset_id), i))
                        .or_insert_with(BigRational::zero) += parse_fraction(value)?;
                }
            }
        }
        if sums.values().any(|sum| sum > &BigRational::one()) {
            return Err(invalid(&format!(
                "La somme des droits dépasse 100 % au {date}. La modification entière est refusée."
            )));
        }
    }
    Ok(())
}
