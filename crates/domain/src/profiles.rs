use crate::*;
use std::collections::BTreeMap;

/// Overlay only descriptive metadata. Dated rights, methods and membership
/// remain independent source records and are never inferred from these profiles.
pub fn apply_profile_periods(snapshot: &RegistrySnapshot, date: &str) -> Result<RegistrySnapshot> {
    validate_registry(snapshot)?;
    validate_date(date)?;
    let mut out = snapshot.clone();
    let mut entity_sources = BTreeMap::new();
    for source in snapshot.entities.iter().chain(&snapshot.profile_revisions) {
        entity_sources
            .entry((source.id.as_str(), source.revision_id.as_str()))
            .or_insert(source);
    }
    let mut asset_sources = BTreeMap::new();
    for source in snapshot
        .assets
        .iter()
        .chain(&snapshot.asset_profile_revisions)
    {
        asset_sources
            .entry((source.id.as_str(), source.revision_id.as_str()))
            .or_insert(source);
    }
    let entity_positions: BTreeMap<_, _> = snapshot
        .entities
        .iter()
        .enumerate()
        .map(|(index, entity)| (entity.id.as_str(), index))
        .collect();
    let asset_positions: BTreeMap<_, _> = snapshot
        .assets
        .iter()
        .enumerate()
        .map(|(index, asset)| (asset.id.as_str(), index))
        .collect();
    for period in snapshot
        .profile_periods
        .iter()
        .filter(|p| applies(&p.valid_from, p.valid_to.as_deref(), date))
    {
        match period.target_kind.as_str() {
            "entity" => {
                let source = entity_sources
                    .get(&(
                        period.target_id.as_str(),
                        period.profile_revision_id.as_str(),
                    ))
                    .expect("validated profile reference");
                if let Some(&index) = entity_positions.get(period.target_id.as_str()) {
                    out.entities[index] = (*source).clone();
                }
            }
            "asset" => {
                let source = asset_sources
                    .get(&(
                        period.target_id.as_str(),
                        period.profile_revision_id.as_str(),
                    ))
                    .expect("validated profile reference");
                if let Some(&index) = asset_positions.get(period.target_id.as_str()) {
                    out.assets[index] = (*source).clone();
                }
            }
            _ => unreachable!("validated profile kind"),
        }
    }
    Ok(out)
}

pub(crate) fn profile_is_dated(s: &RegistrySnapshot, kind: &str, id: &str, date: &str) -> bool {
    s.profile_periods.iter().any(|p| {
        p.target_kind == kind
            && p.target_id == id
            && applies(&p.valid_from, p.valid_to.as_deref(), date)
    })
}
