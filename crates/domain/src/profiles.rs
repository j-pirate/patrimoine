use crate::*;

/// Overlay only descriptive metadata. Dated rights, methods and membership
/// remain independent source records and are never inferred from these profiles.
pub fn apply_profile_periods(snapshot: &RegistrySnapshot, date: &str) -> Result<RegistrySnapshot> {
    validate_registry(snapshot)?;
    validate_date(date)?;
    let mut out = snapshot.clone();
    for period in snapshot
        .profile_periods
        .iter()
        .filter(|p| applies(&p.valid_from, p.valid_to.as_deref(), date))
    {
        match period.target_kind.as_str() {
            "entity" => {
                let source = snapshot
                    .entities
                    .iter()
                    .chain(&snapshot.profile_revisions)
                    .find(|e| {
                        e.id == period.target_id && e.revision_id == period.profile_revision_id
                    })
                    .expect("validated profile reference");
                if let Some(current) = out.entities.iter_mut().find(|e| e.id == period.target_id) {
                    *current = source.clone();
                }
            }
            "asset" => {
                let source = snapshot
                    .assets
                    .iter()
                    .chain(&snapshot.asset_profile_revisions)
                    .find(|a| {
                        a.id == period.target_id && a.revision_id == period.profile_revision_id
                    })
                    .expect("validated profile reference");
                if let Some(current) = out.assets.iter_mut().find(|a| a.id == period.target_id) {
                    *current = source.clone();
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
