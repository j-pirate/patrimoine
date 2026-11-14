use crate::{db, files::*, Vault};
use patrimoine_application::RegistryService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, Row, Transaction};

fn fraction(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Fraction>> {
    let num: Option<String> = row.get(index)?;
    let den: Option<String> = row.get(index + 1)?;
    match (num, den) {
        (Some(num), Some(den)) => Ok(Some(Fraction { num, den })),
        (None, None) => Ok(None),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
fn pairs(f: &Option<Fraction>) -> (Option<&str>, Option<&str>) {
    match f {
        Some(v) => (Some(&v.num), Some(&v.den)),
        None => (None, None),
    }
}
fn normalize_share(share: &mut Option<Fraction>) -> Result<()> {
    if let Some(value) = share {
        *value = exact::normalize_fraction(value)?;
    }
    Ok(())
}
fn entity(row: &Row<'_>) -> rusqlite::Result<Entity> {
    Ok(Entity {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        kind: row.get(2)?,
        display_name: row.get(3)?,
        first_name: row.get(4)?,
        last_name: row.get(5)?,
        birth_date: row.get(6)?,
        residence_country: row.get(7)?,
        preferred_currency: row.get(8)?,
        activity: row.get(9)?,
        notes: row.get(10)?,
        archived: row.get(11)?,
    })
}
fn asset(row: &Row<'_>) -> rusqlite::Result<Asset> {
    Ok(Asset {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        name: row.get(2)?,
        country: row.get(3)?,
        category: row.get(4)?,
        notes: row.get(5)?,
        archived: row.get(6)?,
    })
}
fn phase_four(c: &Connection) -> Result<bool> {
    Ok(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .map_err(db::error)?
            >= 3,
    )
}
pub(crate) fn profile_history<T>(
    c: &Connection,
    table: &str,
    columns: &str,
    joins: &str,
    cutoff: u64,
    reader: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let sql = format!("SELECT r.object_id,r.id,{columns} FROM {table} v JOIN revisions r ON v.revision_id=r.id JOIN change_sets cs ON r.change_set_id=cs.id {joins} WHERE cs.sequence<=?1 AND r.is_void=0 ORDER BY cs.sequence,r.object_id,r.id");
    let mut statement = c.prepare(&sql).map_err(db::error)?;
    let result = statement
        .query_map([cutoff], reader)
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error);
    result
}
pub(crate) fn load<T>(
    c: &Connection,
    table: &str,
    columns: &str,
    joins: &str,
    cutoff: u64,
    reader: impl FnMut(&Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>> {
    let sql=format!("SELECT r.object_id,r.id,{columns} FROM {table} v JOIN revisions r ON v.revision_id=r.id JOIN change_sets cs ON r.change_set_id=cs.id {joins} WHERE cs.sequence<=?1 AND r.is_void=0 AND NOT EXISTS(SELECT 1 FROM revisions next JOIN change_sets nc ON next.change_set_id=nc.id WHERE next.supersedes_revision_id=r.id AND nc.sequence<=?1) ORDER BY r.object_id");
    let mut statement = c.prepare(&sql).map_err(db::error)?;
    let result = statement
        .query_map([cutoff], reader)
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error);
    result
}
pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<RegistrySnapshot> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    let has_phase_four = phase_four(c)?;
    Ok(RegistrySnapshot {
        entities:load(c,"entity_profiles","e.kind,v.display_name,v.first_name,v.last_name,v.birth_date,v.residence_country,v.preferred_currency,v.activity,v.notes,v.archived","JOIN entities e ON e.id=v.entity_id",k,entity)?,
        memberships:load(c,"family_memberships","v.person_id,v.valid_from,v.valid_to,v.relationship","",k,|r|Ok(Membership{id:r.get(0)?,revision_id:r.get(1)?,person_id:r.get(2)?,valid_from:r.get(3)?,valid_to:r.get(4)?,relationship:r.get(5)?}))?,
        participations:load(c,"company_participations","v.owner_entity_id,v.company_id,v.legal_share_num,v.legal_share_den,v.economic_share_num,v.economic_share_den,v.voting_share_num,v.voting_share_den,v.valid_from,v.valid_to","",k,|r|Ok(Participation{id:r.get(0)?,revision_id:r.get(1)?,owner_entity_id:r.get(2)?,company_id:r.get(3)?,legal_share:fraction(r,4)?,economic_share:fraction(r,6)?,voting_share:fraction(r,8)?,valid_from:r.get(10)?,valid_to:r.get(11)?}))?,
        assets:load(c,"asset_profiles","v.name,v.country,v.category,v.notes,v.archived","",k,asset)?,
        ownerships:load(c,"asset_ownerships","v.asset_id,v.owner_entity_id,v.legal_share_num,v.legal_share_den,v.economic_share_num,v.economic_share_den,v.valid_from,v.valid_to","",k,|r|Ok(Ownership{id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,owner_entity_id:r.get(3)?,legal_share:fraction(r,4)?,economic_share:fraction(r,6)?,valid_from:r.get(8)?,valid_to:r.get(9)?}))?,
        valuations:load(c,"valuations","v.target_kind,v.target_id,v.amount,v.currency,v.effective_on,v.source","",k,|r|Ok(Valuation{id:r.get(0)?,revision_id:r.get(1)?,target_kind:r.get(2)?,target_id:r.get(3)?,amount:r.get(4)?,currency:r.get(5)?,effective_on:r.get(6)?,source:r.get(7)?}))?,
        company_policies:load(c,"company_policies","v.company_id,v.mode,v.coverage_complete,v.liability_qualified,v.valid_from,v.valid_to","",k,|r|Ok(CompanyPolicy{id:r.get(0)?,revision_id:r.get(1)?,company_id:r.get(2)?,mode:r.get(3)?,coverage_complete:r.get(4)?,liability_qualified:r.get(5)?,valid_from:r.get(6)?,valid_to:r.get(7)?}))?,
        exchange_rates: if has_phase_four { load(c,"exchange_rates","v.base_currency,v.quote_currency,v.rate,v.effective_on,v.source,v.notes","",k,|r|Ok(ExchangeRate{id:r.get(0)?,revision_id:r.get(1)?,base_currency:r.get(2)?,quote_currency:r.get(3)?,rate:r.get(4)?,effective_on:r.get(5)?,source:r.get(6)?,notes:r.get(7)?}))? } else { Vec::new() },
        profile_periods: if has_phase_four { load(c,"profile_periods","v.target_kind,v.target_id,v.profile_revision_id,v.valid_from,v.valid_to","",k,|r|Ok(ProfilePeriod{id:r.get(0)?,revision_id:r.get(1)?,target_kind:r.get(2)?,target_id:r.get(3)?,profile_revision_id:r.get(4)?,valid_from:r.get(5)?,valid_to:r.get(6)?}))? } else { Vec::new() },
        profile_revisions: profile_history(c,"entity_profiles","e.kind,v.display_name,v.first_name,v.last_name,v.birth_date,v.residence_country,v.preferred_currency,v.activity,v.notes,v.archived","JOIN entities e ON e.id=v.entity_id",k,entity)?,
        asset_profile_revisions: profile_history(c,"asset_profiles","v.name,v.country,v.category,v.notes,v.archived","",k,asset)?,
        knowledge_sequence:k,
    })
}
fn table(kind: &str) -> Result<&'static str> {
    match kind {
        "entity" => Ok("entity_profiles"),
        "membership" => Ok("family_memberships"),
        "participation" => Ok("company_participations"),
        "asset" => Ok("asset_profiles"),
        "ownership" => Ok("asset_ownerships"),
        "valuation" => Ok("valuations"),
        "company_policy" => Ok("company_policies"),
        "exchange_rate" => Ok("exchange_rates"),
        "profile_period" => Ok("profile_periods"),
        "property" => Ok("property_details"),
        "rental_unit" => Ok("rental_units"),
        "property_event" => Ok("property_events"),
        "lease" => Ok("leases"),
        "lease_terms" => Ok("lease_terms"),
        "rent_due" => Ok("rent_dues"),
        "rent_payment" => Ok("rent_payments"),
        "rent_allocation" => Ok("rent_allocations"),
        "rental_expense" => Ok("rental_expenses"),
        "treasury_account" => Ok("treasury_accounts"),
        "treasury_transaction" => Ok("treasury_transactions"),
        "account_reconciliation" => Ok("account_reconciliations"),
        "cash_link" => Ok("cash_links"),
        "deposit_movement" => Ok("deposit_movements"),
        "debt_contract" => Ok("debt_contracts"),
        "debt_terms" => Ok("debt_terms"),
        "debt_movement" => Ok("debt_movements"),
        "debt_receivable_adjustment" => Ok("debt_receivable_adjustments"),
        "debt_schedule" => Ok("debt_schedules"),
        "debt_payment_allocation" => Ok("debt_payment_allocations"),
        "debt_asset_link" => Ok("debt_asset_links"),
        "debt_guarantee" => Ok("debt_guarantees"),
        "debt_coverage" => Ok("debt_coverages"),
        "finance_coverage" => Ok("finance_coverages"),
        "rent_claim" => Ok("rent_claims"),
        "rent_claim_adjustment" => Ok("rent_claim_adjustments"),
        "metal_lot" => Ok("metal_lots"),
        "metal_movement" => Ok("metal_movements"),
        "metal_quote" => Ok("metal_quotes"),
        "metal_premium" => Ok("metal_premiums"),
        "metal_cash_link" => Ok("metal_cash_links"),
        "company_financial_period" => Ok("company_financial_periods"),
        "company_distribution" => Ok("company_distributions"),
        _ => Err(AppError::new(
            "validation",
            "Ce type d’objet ne peut pas être annulé dans le registre.",
        )),
    }
}
pub(crate) fn begin(
    c: &Connection,
    object_id: Option<&str>,
    expected: Option<&str>,
    kind: &str,
    reason: &str,
    is_void: bool,
) -> Result<(String, String)> {
    begin_shared(c, object_id, expected, kind, reason, is_void, None)
}
pub(crate) fn begin_shared(
    c: &Connection,
    object_id: Option<&str>,
    expected: Option<&str>,
    kind: &str,
    reason: &str,
    is_void: bool,
    shared_change: Option<(&str, &str)>,
) -> Result<(String, String)> {
    if reason.trim().is_empty() || reason.len() > 2000 {
        return Err(AppError::new(
            "validation",
            "Indiquez un motif de modification entre 1 et 2 000 octets.",
        ));
    }
    let (object, parent) = match object_id {
        None => {
            if expected.is_some() {
                return Err(AppError::new(
                    "conflict",
                    "Une création ne doit pas désigner une révision précédente.",
                ));
            }
            (id(), None)
        }
        Some(object) => {
            if !valid_id(object) {
                return Err(AppError::new(
                    "validation",
                    "L’identifiant d’objet est invalide.",
                ));
            }
            let mut s=c.prepare("SELECT r.id,o.kind FROM revisions r JOIN objects o ON o.id=r.object_id WHERE r.object_id=?1 AND NOT EXISTS(SELECT 1 FROM revisions n WHERE n.supersedes_revision_id=r.id)").map_err(db::error)?;
            let mut rows = s.query([object]).map_err(db::error)?;
            let row = rows
                .next()
                .map_err(db::error)?
                .ok_or_else(|| AppError::new("not_found", "Cette fiche n’existe plus."))?;
            let previous: String = row.get(0).map_err(db::error)?;
            let previous_kind: String = row.get(1).map_err(db::error)?;
            if rows.next().map_err(db::error)?.is_some()
                || expected != Some(previous.as_str())
                || previous_kind != kind
            {
                return Err(AppError::new(
                    "conflict",
                    "La fiche a changé. Rechargez-la avant de modifier sa révision.",
                ));
            }
            (object.to_owned(), Some(previous))
        }
    };
    let (change, stamp) = match shared_change {
        Some((change, stamp)) => (change.to_owned(), stamp.to_owned()),
        None => db::mutation(c, reason.trim())?,
    };
    if parent.is_none() {
        c.execute(
            "INSERT INTO objects VALUES(?1,?2,?3)",
            params![object, kind, stamp],
        )
        .map_err(db::error)?;
    }
    let revision = id();
    c.execute(
        "INSERT INTO revisions VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            revision,
            object,
            change,
            stamp,
            parent,
            is_void,
            reason.trim()
        ],
    )
    .map_err(db::error)?;
    c.execute(
        "INSERT INTO audit_events VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            id(),
            change,
            object,
            revision,
            if is_void {
                "registry_voided"
            } else {
                "registry_saved"
            },
            reason.trim()
        ],
    )
    .map_err(db::error)?;
    Ok((object, revision))
}
impl Vault {
    fn write_registry(
        &mut self,
        kind: &str,
        object: Option<&str>,
        expected: Option<&str>,
        reason: &str,
        write: impl FnOnce(&Transaction<'_>, &str, &str) -> Result<()>,
    ) -> Result<String> {
        self.ensure_active()?;
        let mut connection = db::connect(&self.db_path())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let (object, revision) = begin(&tx, object, expected, kind, reason, false)?;
        write(&tx, &object, &revision)?;
        validate_profile_sources(&tx)?;
        validate_registry(&snapshot(&tx, None)?)?;
        crate::properties::validate_current(&tx)?;
        crate::rentals::validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        tx.commit().map_err(db::error)?;
        Ok(object)
    }
}
impl RegistryService for Vault {
    fn list_registry(&self) -> Result<RegistrySnapshot> {
        self.list_registry_at(None)
    }
    fn list_registry_at(&self, knowledge_sequence: Option<u64>) -> Result<RegistrySnapshot> {
        self.ensure_active()?;
        let mut connection = db::readonly(&self.db_path())?;
        let transaction = connection.transaction().map_err(db::error)?;
        snapshot(&transaction, knowledge_sequence)
    }
    fn list_registry_history(&self) -> Result<RegistryHistory> {
        self.ensure_active()?;
        let mut connection = db::readonly(&self.db_path())?;
        let tx = connection.transaction().map_err(db::error)?;
        let latest_sequence = db::sequence(&tx)?;
        let mut query = tx.prepare("SELECT cs.sequence,r.recorded_at,r.object_id,r.id,o.kind,COALESCE(r.reason,cs.reason),r.is_void,CASE o.kind WHEN 'entity' THEN (SELECT display_name FROM entity_profiles WHERE revision_id=r.id) WHEN 'asset' THEN (SELECT name FROM asset_profiles WHERE revision_id=r.id) WHEN 'exchange_rate' THEN (SELECT base_currency||' → '||quote_currency||' · '||effective_on FROM exchange_rates WHERE revision_id=r.id) WHEN 'profile_period' THEN (SELECT target_kind||' · '||valid_from FROM profile_periods WHERE revision_id=r.id) WHEN 'membership' THEN (SELECT relationship||' · '||valid_from FROM family_memberships WHERE revision_id=r.id) WHEN 'participation' THEN 'Participation' WHEN 'ownership' THEN 'Détention' WHEN 'valuation' THEN (SELECT amount||' '||currency||' · '||effective_on FROM valuations WHERE revision_id=r.id) WHEN 'company_policy' THEN (SELECT mode||' · '||valid_from FROM company_policies WHERE revision_id=r.id) WHEN 'document' THEN (SELECT original_name FROM document_versions WHERE revision_id=r.id) WHEN 'document_link' THEN (SELECT COALESCE(role,'Association documentaire')||' · '||target_object_id FROM document_links WHERE revision_id=r.id) ELSE o.kind END FROM revisions r JOIN change_sets cs ON cs.id=r.change_set_id JOIN objects o ON o.id=r.object_id WHERE o.kind IN ('entity','membership','participation','asset','ownership','valuation','company_policy','exchange_rate','profile_period','property','rental_unit','property_event','lease','lease_terms','rent_due','rent_payment','rent_allocation','rental_expense','treasury_account','treasury_transaction','account_reconciliation','cash_link','deposit_movement','debt_contract','debt_terms','debt_movement','debt_receivable_adjustment','debt_schedule','debt_payment_allocation','debt_asset_link','debt_guarantee','debt_coverage','finance_coverage','rent_claim','rent_claim_adjustment','metal_lot','metal_movement','metal_quote','metal_premium','metal_cash_link','company_financial_period','company_distribution','document','document_link') ORDER BY cs.sequence DESC,r.object_id,r.id").map_err(db::error)?;
        let events = query
            .query_map([], |r| {
                Ok(RegistryHistoryEvent {
                    sequence: r.get(0)?,
                    recorded_at: r.get(1)?,
                    object_id: r.get(2)?,
                    revision_id: r.get(3)?,
                    kind: r.get(4)?,
                    reason: r.get(5)?,
                    is_void: r.get(6)?,
                    label: r.get(7)?,
                })
            })
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        Ok(RegistryHistory {
            latest_sequence,
            events,
        })
    }
    fn get_consolidation(&self, request: ConsolidationRequest) -> Result<ConsolidationReport> {
        self.ensure_active()?;
        let mut connection = db::readonly(&self.db_path())?;
        let transaction = connection.transaction().map_err(db::error)?;
        let snapshot = snapshot(&transaction, request.knowledge_sequence)?;
        consolidate_with_metals(
            &snapshot,
            &crate::properties::snapshot(&transaction, request.knowledge_sequence)?,
            &crate::treasury::snapshot(&transaction, request.knowledge_sequence)?,
            &crate::metals::snapshot(&transaction, request.knowledge_sequence)?,
            &request,
        )
    }
    fn convert_currency(&self, mut query: FxQuery) -> Result<FxResult> {
        query.amount = exact::normalize_decimal_input(&query.amount)?;
        let state = self.list_registry_at(query.knowledge_sequence)?;
        patrimoine_domain::convert_currency(&state, &query)
    }
    fn explain_variation(&self, request: VariationRequest) -> Result<VariationReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let mut state = snapshot(&tx, request.knowledge_sequence)?;
        let metals = crate::metals::snapshot(&tx, request.knowledge_sequence)?;
        let assets = metals
            .lots
            .iter()
            .map(|l| l.asset_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        state
            .valuations
            .retain(|v| v.target_kind != "asset" || !assets.contains(v.target_id.as_str()));
        let mut report = patrimoine_domain::explain_variation(&state, &request)?;
        if !assets.is_empty() {
            report.status = "partial".into();
            report.warnings.push(ConsolidationWarning {code:"metal_stocks_excluded_from_bridge".into(),object_id:"scope".into(),message:"Ce pont historique porte sur les estimations du registre. Les stocks métal calculés sont exclus ; leur ancienne estimation générique n’a pas été réutilisée et aucune performance globale n’est déduite.".into()});
        }
        Ok(report)
    }
    fn save_entity(&mut self, i: SaveEntityInput) -> Result<Entity> {
        let key=self.write_registry("entity",i.id.as_deref(),i.expected_revision_id.as_deref(),&i.reason,|tx,key,rev|{
            tx.execute("INSERT OR IGNORE INTO entities VALUES(?1,?2)",params![key,i.kind]).map_err(db::error)?;
            let existing:String=tx.query_row("SELECT kind FROM entities WHERE id=?1",[key],|r|r.get(0)).map_err(db::error)?;
            if existing!=i.kind{return Err(AppError::new("validation","La nature personne ou société d’une identité existante ne peut pas changer."));}
            tx.execute("INSERT INTO entity_profiles VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",params![rev,key,i.display_name,i.first_name,i.last_name,i.birth_date,i.residence_country,i.preferred_currency,i.activity,i.notes,i.archived]).map_err(db::error)?;Ok(())})?;
        self.list_registry()?
            .entities
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("La fiche enregistrée est introuvable."))
    }
    fn save_membership(&mut self, i: SaveMembershipInput) -> Result<Membership> {
        let key = self.write_registry(
            "membership",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO family_memberships VALUES(?1,?2,?3,?4,?5)",
                    params![rev, i.person_id, i.valid_from, i.valid_to, i.relationship],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .memberships
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("L’appartenance enregistrée est introuvable."))
    }
    fn save_participation(&mut self, mut i: SaveParticipationInput) -> Result<Participation> {
        normalize_share(&mut i.legal_share)?;
        normalize_share(&mut i.economic_share)?;
        normalize_share(&mut i.voting_share)?;
        let key = self.write_registry(
            "participation",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                let (ln, ld) = pairs(&i.legal_share);
                let (en, ed) = pairs(&i.economic_share);
                let (vn, vd) = pairs(&i.voting_share);
                tx.execute(
                    "INSERT INTO company_participations VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                    params![
                        rev,
                        i.owner_entity_id,
                        i.company_id,
                        ln,
                        ld,
                        en,
                        ed,
                        vn,
                        vd,
                        i.valid_from,
                        i.valid_to
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .participations
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("La participation enregistrée est introuvable."))
    }
    fn save_asset(&mut self, i: SaveAssetInput) -> Result<Asset> {
        let key = self.write_registry(
            "asset",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, key, rev| {
                tx.execute("INSERT OR IGNORE INTO assets VALUES(?1)", [key])
                    .map_err(db::error)?;
                tx.execute(
                    "INSERT INTO asset_profiles VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![rev, key, i.name, i.country, i.category, i.notes, i.archived],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .assets
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("L’actif enregistré est introuvable."))
    }
    fn save_ownership(&mut self, mut i: SaveOwnershipInput) -> Result<Ownership> {
        normalize_share(&mut i.legal_share)?;
        normalize_share(&mut i.economic_share)?;
        let key = self.write_registry(
            "ownership",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                let (ln, ld) = pairs(&i.legal_share);
                let (en, ed) = pairs(&i.economic_share);
                tx.execute(
                    "INSERT INTO asset_ownerships VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        rev,
                        i.asset_id,
                        i.owner_entity_id,
                        ln,
                        ld,
                        en,
                        ed,
                        i.valid_from,
                        i.valid_to
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .ownerships
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("Le droit enregistré est introuvable."))
    }
    fn save_valuation(&mut self, mut i: SaveValuationInput) -> Result<Valuation> {
        i.amount = exact::normalize_decimal_input(&i.amount)?;
        let key = self.write_registry(
            "valuation",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO valuations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        rev,
                        i.target_kind,
                        i.target_id,
                        i.amount,
                        i.currency,
                        i.effective_on,
                        i.source
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .valuations
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("La valorisation enregistrée est introuvable."))
    }
    fn save_company_policy(&mut self, i: SaveCompanyPolicyInput) -> Result<CompanyPolicy> {
        let key = self.write_registry(
            "company_policy",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO company_policies VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        rev,
                        i.company_id,
                        i.mode,
                        i.coverage_complete,
                        i.liability_qualified,
                        i.valid_from,
                        i.valid_to
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .company_policies
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("La politique enregistrée est introuvable."))
    }
    fn save_exchange_rate(&mut self, mut i: SaveExchangeRateInput) -> Result<ExchangeRate> {
        i.rate = exact::normalize_rate_input(&i.rate)?;
        let key = self.write_registry(
            "exchange_rate",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO exchange_rates VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        rev,
                        i.base_currency,
                        i.quote_currency,
                        i.rate,
                        i.effective_on,
                        i.source,
                        i.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .exchange_rates
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("Le taux enregistré est introuvable."))
    }
    fn save_profile_period(&mut self, i: SaveProfilePeriodInput) -> Result<ProfilePeriod> {
        let key = self.write_registry(
            "profile_period",
            i.id.as_deref(),
            i.expected_revision_id.as_deref(),
            &i.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO profile_periods VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        rev,
                        i.target_kind,
                        i.target_id,
                        i.profile_revision_id,
                        i.valid_from,
                        i.valid_to
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_registry()?
            .profile_periods
            .into_iter()
            .find(|v| v.id == key)
            .ok_or_else(|| invalid("La période de profil enregistrée est introuvable."))
    }
    fn void_registry_record(&mut self, input: VoidRecordInput) -> Result<()> {
        self.ensure_active()?;
        let mut connection = db::connect(&self.db_path())?;
        let tx = connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let kind: String = tx
            .query_row("SELECT kind FROM objects WHERE id=?1", [&input.id], |r| {
                r.get(0)
            })
            .map_err(db::error)?;
        let table = table(&kind)?;
        if kind == "property_event" {
            let event_kind: String = tx
                .query_row(
                    "SELECT kind FROM property_events WHERE revision_id=?1",
                    [&input.expected_revision_id],
                    |r| r.get(0),
                )
                .map_err(db::error)?;
            if event_kind == "disposal" {
                return Err(AppError::new("validation","Une cession ne peut pas être annulée isolément de la clôture des droits. Enregistrez une régularisation explicite."));
            }
        }
        let (_, revision) = begin(
            &tx,
            Some(&input.id),
            Some(&input.expected_revision_id),
            &kind,
            &input.reason,
            true,
        )?;
        let mut statement = tx
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db::error)?;
        let columns = statement
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        drop(statement);
        let rest = columns
            .iter()
            .skip(1)
            .map(|v| format!("\"{v}\""))
            .collect::<Vec<_>>()
            .join(",");
        tx.execute(
            &format!("INSERT INTO {table} SELECT ?1,{rest} FROM {table} WHERE revision_id=?2"),
            params![revision, input.expected_revision_id],
        )
        .map_err(db::error)?;
        match kind.as_str() {
            "treasury_transaction" => {
                tx.execute("INSERT INTO treasury_entries SELECT ?1,line_id,account_id,amount,role FROM treasury_entries WHERE transaction_revision_id=?2",params![revision,input.expected_revision_id]).map_err(db::error)?;
            }
            "debt_terms" => {
                tx.execute("INSERT INTO debt_parties SELECT ?1,line_id,side,entity_id,legal_share_num,legal_share_den,economic_share_num,economic_share_den,liability_kind,notes FROM debt_parties WHERE terms_revision_id=?2",params![revision,input.expected_revision_id]).map_err(db::error)?;
            }
            "rent_claim" => {
                tx.execute("INSERT INTO rent_claim_parties SELECT ?1,entity_id,economic_share_num,economic_share_den FROM rent_claim_parties WHERE claim_revision_id=?2",params![revision,input.expected_revision_id]).map_err(db::error)?;
            }
            "company_distribution" => {
                tx.execute("INSERT INTO company_distribution_cash_links SELECT ?1,side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=?2",params![revision,input.expected_revision_id]).map_err(db::error)?;
            }
            _ => {}
        }
        validate_profile_sources(&tx)?;
        validate_registry(&snapshot(&tx, None)?)?;
        crate::properties::validate_current(&tx)?;
        crate::rentals::validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        tx.commit().map_err(db::error)?;
        Ok(())
    }
}
fn validate_profile_sources(c: &Connection) -> Result<()> {
    if !phase_four(c)? {
        return Ok(());
    }
    let invalid_sources:u64 = c.query_row("SELECT count(*) FROM profile_periods v JOIN revisions r ON r.id=v.revision_id JOIN change_sets pc ON pc.id=r.change_set_id JOIN revisions source ON source.id=v.profile_revision_id JOIN change_sets sc ON sc.id=source.change_set_id WHERE source.object_id<>v.target_id OR source.is_void<>0 OR sc.sequence>pc.sequence OR (v.target_kind='entity' AND NOT EXISTS(SELECT 1 FROM entity_profiles p WHERE p.revision_id=source.id AND p.entity_id=v.target_id)) OR (v.target_kind='asset' AND NOT EXISTS(SELECT 1 FROM asset_profiles p WHERE p.revision_id=source.id AND p.asset_id=v.target_id))",[],|r|r.get(0)).map_err(db::error)?;
    if invalid_sources != 0 {
        return Err(invalid("Une période doit désigner une révision de profil connue de la même identité et du même type."));
    }
    Ok(())
}
pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    let identities: u64 = c.query_row(
        "SELECT (SELECT count(*) FROM entity_profiles v JOIN revisions r ON v.revision_id=r.id WHERE v.entity_id<>r.object_id) + (SELECT count(*) FROM asset_profiles v JOIN revisions r ON v.revision_id=r.id WHERE v.asset_id<>r.object_id) + (SELECT count(*) FROM entities e JOIN objects o ON e.id=o.id WHERE o.kind<>'entity') + (SELECT count(*) FROM assets a JOIN objects o ON a.id=o.id WHERE o.kind<>'asset') + (SELECT count(*) FROM objects o WHERE (o.kind='entity' AND NOT EXISTS(SELECT 1 FROM entities e WHERE e.id=o.id)) OR (o.kind='asset' AND NOT EXISTS(SELECT 1 FROM assets a WHERE a.id=o.id))) + (SELECT count(*) FROM objects o WHERE o.kind IN ('entity','membership','participation','asset','ownership','valuation','company_policy','exchange_rate','profile_period','property','rental_unit','property_event','lease','lease_terms','rent_due','rent_payment','rent_allocation','rental_expense','treasury_account','treasury_transaction','account_reconciliation','cash_link','deposit_movement','debt_contract','debt_terms','debt_movement','debt_receivable_adjustment','debt_schedule','debt_payment_allocation','debt_asset_link','debt_guarantee','debt_coverage','finance_coverage','rent_claim','rent_claim_adjustment') AND ((SELECT count(CASE WHEN r.supersedes_revision_id IS NULL THEN 1 END) FROM revisions r WHERE r.object_id=o.id)<>1 OR (SELECT count(*) FROM revisions r WHERE r.object_id=o.id AND NOT EXISTS(SELECT 1 FROM revisions n WHERE n.supersedes_revision_id=r.id))<>1))",
        [], |r| r.get(0),
    ).map_err(db::error)?;
    if identities != 0 {
        return Err(invalid(
            "Les identités, profils ou chaînes de révisions du registre sont incohérents.",
        ));
    }
    validate_profile_sources(c)?;
    let mut sequences = c
        .prepare("SELECT sequence FROM change_sets ORDER BY sequence")
        .map_err(db::error)?;
    for sequence in sequences
        .query_map([], |r| r.get::<_, u64>(0))
        .map_err(db::error)?
    {
        validate_registry(&snapshot(c, Some(sequence.map_err(db::error)?))?)?;
    }
    let mut tables = vec![
        ("entity", "entity_profiles"),
        ("membership", "family_memberships"),
        ("participation", "company_participations"),
        ("asset", "asset_profiles"),
        ("ownership", "asset_ownerships"),
        ("valuation", "valuations"),
        ("company_policy", "company_policies"),
    ];
    if phase_four(c)? {
        tables.extend([
            ("exchange_rate", "exchange_rates"),
            ("profile_period", "profile_periods"),
        ]);
    }
    if crate::properties::enabled(c)? {
        tables.extend([
            ("property", "property_details"),
            ("rental_unit", "rental_units"),
            ("property_event", "property_events"),
            ("lease", "leases"),
            ("lease_terms", "lease_terms"),
            ("rent_due", "rent_dues"),
            ("rent_payment", "rent_payments"),
            ("rent_allocation", "rent_allocations"),
            ("rental_expense", "rental_expenses"),
        ]);
        crate::properties::validate_history(c)?;
        crate::rentals::validate_history(c)?;
    }
    if crate::finance::enabled(c)? {
        crate::finance::validate_history(c)?;
    }
    for (kind, table) in tables {
        let inconsistent:u64=c.query_row(&format!("SELECT (SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN objects o ON o.id=r.object_id WHERE o.kind<>?1) + (SELECT count(*) FROM revisions r JOIN objects o ON o.id=r.object_id WHERE o.kind=?1 AND NOT EXISTS(SELECT 1 FROM {table} v WHERE v.revision_id=r.id))"),[kind],|r|r.get(0)).map_err(db::error)?;
        if inconsistent != 0 {
            return Err(invalid(
                "Une révision du registre ne correspond pas au type de son objet.",
            ));
        }
        let mut columns = c
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db::error)?;
        let comparisons = columns
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?
            .into_iter()
            .skip(1)
            .map(|column| format!("p.\"{column}\" IS v.\"{column}\""))
            .collect::<Vec<_>>()
            .join(" AND ");
        let invalid_voids: u64 = c.query_row(&format!("SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND NOT EXISTS(SELECT 1 FROM {table} p WHERE p.revision_id=r.supersedes_revision_id AND {comparisons})"), [], |r| r.get(0)).map_err(db::error)?;
        if invalid_voids != 0 {
            return Err(invalid(
                "Une annulation du registre doit conserver le contenu de sa révision précédente.",
            ));
        }
    }
    Ok(())
}
