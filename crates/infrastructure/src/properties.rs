use crate::{db, files::*, registry, Vault};
use patrimoine_application::PropertyService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, Transaction, TransactionBehavior};

pub(crate) fn enabled(c: &Connection) -> Result<bool> {
    Ok(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .map_err(db::error)?
            >= 4,
    )
}

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<PropertySnapshot> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    if !enabled(c)? {
        return Ok(PropertySnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    Ok(PropertySnapshot {
        properties: registry::load(c, "property_details", "v.asset_id,v.property_type,v.city,v.address,v.surface_m2,v.status,v.valid_from,v.valid_to,v.notes", "", k, |r| Ok(Property {
            id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,property_type:r.get(3)?,city:r.get(4)?,address:r.get(5)?,surface_m2:r.get(6)?,status:r.get(7)?,valid_from:r.get(8)?,valid_to:r.get(9)?,notes:r.get(10)?,
        }))?,
        units: registry::load(c,"rental_units","v.property_asset_id,v.reference,v.name,v.surface_m2,v.valid_from,v.valid_to,v.available_for_rent,v.notes","",k,|r|Ok(RentalUnit{
            id:r.get(0)?,revision_id:r.get(1)?,property_asset_id:r.get(2)?,reference:r.get(3)?,name:r.get(4)?,surface_m2:r.get(5)?,valid_from:r.get(6)?,valid_to:r.get(7)?,available_for_rent:r.get(8)?,notes:r.get(9)?,
        }))?,
        events:registry::load(c,"property_events","v.asset_id,v.kind,v.effective_on,v.price,v.fees,v.currency,v.notes","",k,|r|Ok(PropertyEvent{
            id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,kind:r.get(3)?,effective_on:r.get(4)?,price:r.get(5)?,fees:r.get(6)?,currency:r.get(7)?,notes:r.get(8)?,
        }))?,
        knowledge_sequence:k,
    })
}

pub(crate) fn validate_current(c: &Connection) -> Result<()> {
    if enabled(c)? {
        validate_properties(&registry::snapshot(c, None)?, &snapshot(c, None)?)?;
    }
    Ok(())
}

fn normalize_optional(value: &mut Option<String>) -> Result<()> {
    if let Some(amount) = value {
        *amount = exact::normalize_decimal_input(amount)?;
    }
    Ok(())
}

pub(crate) fn insert_property(
    c: &Connection,
    revision: &str,
    input: &SavePropertyInput,
) -> Result<()> {
    c.execute(
        "INSERT INTO property_details VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            revision,
            input.asset_id,
            input.property_type,
            input.city,
            input.address,
            input.surface_m2,
            input.status,
            input.valid_from,
            input.valid_to,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

impl Vault {
    fn write_property<T>(
        &mut self,
        kind: &str,
        object: Option<&str>,
        expected: Option<&str>,
        reason: &str,
        write: impl FnOnce(&Transaction<'_>, &str, &str) -> Result<()>,
        read: impl FnOnce(PropertySnapshot, &str) -> Result<T>,
    ) -> Result<T> {
        self.ensure_active()?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let (object, revision) = registry::begin(&tx, object, expected, kind, reason, false)?;
        write(&tx, &object, &revision)?;
        validate_current(&tx)?;
        crate::rentals::validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        validate_identities(&tx)?;
        let result = read(snapshot(&tx, None)?, &object)?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn correct_property_disposal(
        &mut self,
        input: SavePropertyEventInput,
    ) -> Result<PropertyEvent> {
        self.ensure_active()?;
        validate_date(&input.effective_on)?;
        let object=input.id.as_deref().ok_or_else(||AppError::new("validation","Utilisez la commande de cession pour enregistrer une nouvelle vente et clore ses droits."))?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let old = snapshot(&tx, None)?
            .events
            .into_iter()
            .find(|e| e.id == object)
            .ok_or_else(|| {
                AppError::new(
                    "conflict",
                    "La cession n’est plus disponible dans les connaissances actuelles.",
                )
            })?;
        if old.kind != "disposal"
            || old.asset_id != input.asset_id
            || input.expected_revision_id.as_deref() != Some(old.revision_id.as_str())
        {
            return Err(AppError::new("conflict","La cession a changé ou désigne un autre bien. Rechargez sa révision avant de la corriger."));
        }
        let initial_change:String=tx.query_row("SELECT r.change_set_id FROM revisions r JOIN property_events e ON e.revision_id=r.id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.object_id=?1 AND e.kind='disposal' AND r.is_void=0 ORDER BY cs.sequence LIMIT 1",[object],|r|r.get(0)).map_err(db::error)?;
        let mut statement=tx.prepare("SELECT r.object_id,r.supersedes_revision_id FROM revisions r JOIN asset_ownerships o ON o.revision_id=r.id WHERE r.change_set_id=?1 AND o.asset_id=?2 ORDER BY r.object_id").map_err(db::error)?;
        let sources = statement
            .query_map(params![initial_change, input.asset_id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, Option<String>>(1)?))
            })
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        drop(statement);
        if sources.is_empty() {
            return Err(AppError::new("conflict","Les droits d’origine de cette cession ne sont pas identifiables. Aucune correction isolée n’a été effectuée."));
        }
        let mut corrected_rights = Vec::new();
        for (ownership_id, source_revision) in sources {
            let source = read_ownership_revision(
                &tx,
                &source_revision
                    .ok_or_else(|| invalid("La cession ne référence pas ses droits précédents."))?,
            )?;
            let (current_revision,is_void):(String,bool)=tx.query_row("SELECT r.id,r.is_void FROM revisions r WHERE r.object_id=?1 AND NOT EXISTS(SELECT 1 FROM revisions next WHERE next.supersedes_revision_id=r.id)",[&ownership_id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(db::error)?;
            let current = read_ownership_revision(&tx, &current_revision)?;
            if source.asset_id != input.asset_id
                || source.id != current.id
                || source.asset_id != current.asset_id
                || source.owner_entity_id != current.owner_entity_id
                || source.legal_share != current.legal_share
                || source.economic_share != current.economic_share
                || source.valid_from != current.valid_from
                || (is_void && source.valid_from != old.effective_on)
                || (!is_void
                    && (current.valid_to.as_deref() != Some(old.effective_on.as_str())
                        || source.valid_from >= old.effective_on))
            {
                return Err(AppError::new("conflict","Un droit lié à la cession a été modifié séparément. Régularisez ce désaccord avant de corriger la date de vente."));
            }
            if input.effective_on < source.valid_from
                || source
                    .valid_to
                    .as_deref()
                    .is_some_and(|end| input.effective_on.as_str() > end)
            {
                return Err(AppError::new("validation","La date corrigée ne peut pas précéder le début d’un droit ni dépasser sa fin initialement prévue."));
            }
            corrected_rights.push((source, current));
        }
        let (change, stamp) = db::mutation(&tx, input.reason.trim())?;
        if input.effective_on != old.effective_on {
            for (source, current) in corrected_rights {
                let is_void = input.effective_on == source.valid_from;
                let (_, revision) = registry::begin_shared(
                    &tx,
                    Some(&current.id),
                    Some(&current.revision_id),
                    "ownership",
                    &input.reason,
                    is_void,
                    Some((&change, &stamp)),
                )?;
                // A tombstone copies its immediate predecessor byte for byte.
                // Reopening restores the immutable right and a corrected end.
                let mut row = if is_void { current } else { source };
                if !is_void {
                    row.valid_to = Some(input.effective_on.clone());
                }
                insert_ownership(&tx, &revision, &row)?;
            }
        }
        let (_, revision) = registry::begin_shared(
            &tx,
            Some(object),
            input.expected_revision_id.as_deref(),
            "property_event",
            &input.reason,
            false,
            Some((&change, &stamp)),
        )?;
        tx.execute(
            "INSERT INTO property_events VALUES(?1,?2,'disposal',?3,?4,?5,?6,?7)",
            params![
                revision,
                input.asset_id,
                input.effective_on,
                input.price,
                input.fees,
                input.currency,
                input.notes
            ],
        )
        .map_err(db::error)?;
        validate_current(&tx)?;
        crate::rentals::validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        validate_identities(&tx)?;
        let result = snapshot(&tx, None)?
            .events
            .into_iter()
            .find(|e| e.id == object)
            .ok_or_else(|| invalid("La cession corrigée est introuvable."))?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
}

fn read_ownership_revision(c: &Connection, revision: &str) -> Result<Ownership> {
    c.query_row("SELECT r.object_id,r.id,v.asset_id,v.owner_entity_id,v.legal_share_num,v.legal_share_den,v.economic_share_num,v.economic_share_den,v.valid_from,v.valid_to FROM asset_ownerships v JOIN revisions r ON r.id=v.revision_id WHERE r.id=?1",[revision],|r|{
        let legal_num:Option<String>=r.get(4)?; let legal_den:Option<String>=r.get(5)?;
        let economic_num:Option<String>=r.get(6)?; let economic_den:Option<String>=r.get(7)?;
        Ok(Ownership{id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,owner_entity_id:r.get(3)?,legal_share:legal_num.zip(legal_den).map(|(num,den)|Fraction{num,den}),economic_share:economic_num.zip(economic_den).map(|(num,den)|Fraction{num,den}),valid_from:r.get(8)?,valid_to:r.get(9)?})
    }).map_err(db::error)
}

fn insert_ownership(c: &Connection, revision: &str, row: &Ownership) -> Result<()> {
    let legal = row.legal_share.as_ref();
    let economic = row.economic_share.as_ref();
    c.execute(
        "INSERT INTO asset_ownerships VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            revision,
            row.asset_id,
            row.owner_entity_id,
            legal.map(|x| x.num.as_str()),
            legal.map(|x| x.den.as_str()),
            economic.map(|x| x.num.as_str()),
            economic.map(|x| x.den.as_str()),
            row.valid_from,
            row.valid_to
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

impl PropertyService for Vault {
    fn list_properties_at(&self, knowledge_sequence: Option<u64>) -> Result<PropertySnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_property(&mut self, mut input: SavePropertyInput) -> Result<Property> {
        normalize_optional(&mut input.surface_m2)?;
        self.write_property(
            "property",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| insert_property(tx, rev, &input),
            |saved, key| {
                saved
                    .properties
                    .into_iter()
                    .find(|p| p.id == key)
                    .ok_or_else(|| invalid("La fiche immobilière enregistrée est introuvable."))
            },
        )
    }
    fn save_rental_unit(&mut self, mut input: SaveRentalUnitInput) -> Result<RentalUnit> {
        normalize_optional(&mut input.surface_m2)?;
        self.write_property(
            "rental_unit",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO rental_units VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        rev,
                        input.property_asset_id,
                        input.reference,
                        input.name,
                        input.surface_m2,
                        input.valid_from,
                        input.valid_to,
                        input.available_for_rent,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |saved, key| {
                saved
                    .units
                    .into_iter()
                    .find(|p| p.id == key)
                    .ok_or_else(|| invalid("L’unité enregistrée est introuvable."))
            },
        )
    }
    fn save_property_event(&mut self, mut input: SavePropertyEventInput) -> Result<PropertyEvent> {
        normalize_optional(&mut input.price)?;
        normalize_optional(&mut input.fees)?;
        if input.kind == "disposal" {
            return self.correct_property_disposal(input);
        }
        self.write_property("property_event",input.id.as_deref(),input.expected_revision_id.as_deref(),&input.reason,|tx,_,rev|{
            if let Some(previous)=&input.expected_revision_id {
                let previous_kind:String=tx.query_row("SELECT kind FROM property_events WHERE revision_id=?1",[previous],|r|r.get(0)).map_err(db::error)?;
                if previous_kind=="disposal" { return Err(AppError::new("validation","Une cession ne peut pas être modifiée isolément de la clôture des droits. Enregistrez une régularisation explicite.")); }
            }
            tx.execute("INSERT INTO property_events VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",params![rev,input.asset_id,input.kind,input.effective_on,input.price,input.fees,input.currency,input.notes]).map_err(db::error)?;
            Ok(())
        }, |saved, key| saved.events.into_iter().find(|p| p.id == key).ok_or_else(|| invalid("L’événement enregistré est introuvable.")))
    }
    fn get_property_report(&self, request: PropertyReportRequest) -> Result<PropertyReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let k = request.knowledge_sequence;
        financial_property_report(
            &registry::snapshot(&tx, k)?,
            &snapshot(&tx, k)?,
            &crate::rentals::snapshot(&tx, k)?,
            &crate::treasury::snapshot(&tx, k)?,
            &crate::debts::snapshot(&tx, k)?,
            &crate::finance::snapshot(&tx, k)?,
            &request,
        )
    }
    fn dispose_property(&mut self, mut input: DisposePropertyInput) -> Result<PropertySnapshot> {
        self.ensure_active()?;
        validate_date(&input.effective_on)?;
        normalize_optional(&mut input.price)?;
        normalize_optional(&mut input.fees)?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        if db::sequence(&tx)? != input.expected_sequence {
            return Err(AppError::new(
                "conflict",
                "Le coffre a changé. Rechargez-le avant de confirmer la cession.",
            ));
        }
        let before = registry::snapshot(&tx, None)?;
        let property = snapshot(&tx, None)?;
        if !property
            .properties
            .iter()
            .any(|p| p.asset_id == input.asset_id)
        {
            return Err(AppError::new(
                "validation",
                "La cession exige une fiche immobilière enregistrée.",
            ));
        }
        let rights: Vec<_> = before
            .ownerships
            .iter()
            .filter(|o| {
                o.asset_id == input.asset_id
                    && o.valid_to
                        .as_deref()
                        .is_none_or(|end| end > input.effective_on.as_str())
            })
            .collect();
        if rights.is_empty() {
            return Err(AppError::new(
                "validation",
                "Aucun droit suivi n’est à clore à cette date.",
            ));
        }
        if rights.iter().any(|o| o.valid_from > input.effective_on) {
            return Err(AppError::new("validation","Des droits futurs existent sur ce bien. Régularisez-les avant la cession complète."));
        }
        let (change, stamp) = db::mutation(&tx, &input.reason)?;
        for old in rights {
            let is_void = old.valid_from == input.effective_on;
            let (_, rev) = registry::begin_shared(
                &tx,
                Some(&old.id),
                Some(&old.revision_id),
                "ownership",
                &input.reason,
                is_void,
                Some((&change, &stamp)),
            )?;
            let end = if is_void {
                old.valid_to.as_deref()
            } else {
                Some(input.effective_on.as_str())
            };
            let legal = old.legal_share.as_ref();
            let economic = old.economic_share.as_ref();
            tx.execute(
                "INSERT INTO asset_ownerships VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    rev,
                    old.asset_id,
                    old.owner_entity_id,
                    legal.map(|x| x.num.as_str()),
                    legal.map(|x| x.den.as_str()),
                    economic.map(|x| x.num.as_str()),
                    economic.map(|x| x.den.as_str()),
                    old.valid_from,
                    end
                ],
            )
            .map_err(db::error)?;
        }
        let (_, revision) = registry::begin_shared(
            &tx,
            None,
            None,
            "property_event",
            &input.reason,
            false,
            Some((&change, &stamp)),
        )?;
        tx.execute(
            "INSERT INTO property_events VALUES(?1,?2,'disposal',?3,?4,?5,?6,?7)",
            params![
                revision,
                input.asset_id,
                input.effective_on,
                input.price,
                input.fees,
                input.currency,
                "Cession complète : droits suivis clos atomiquement ; aucun flux bancaire créé."
            ],
        )
        .map_err(db::error)?;
        validate_current(&tx)?;
        crate::rentals::validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        validate_identities(&tx)?;
        let result = snapshot(&tx, None)?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
}

pub(crate) fn validate_identities(c: &Connection) -> Result<()> {
    for (table, key) in [
        ("property_details", "asset_id"),
        ("rental_units", "property_asset_id"),
        ("property_events", "asset_id"),
    ] {
        let changed:u64=c.query_row(&format!("SELECT count(*) FROM (SELECT r.object_id FROM {table} v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.{key})>1)"),[],|r|r.get(0)).map_err(db::error)?;
        if changed != 0 {
            return Err(invalid(
                "Une identité immobilière ne peut pas changer de bien entre deux révisions.",
            ));
        }
    }
    Ok(())
}

pub(crate) const KINDS: &[&str] = &["property", "rental_unit", "property_event"];

pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    validate_identities(c)?;
    let kinds = [registry::KINDS, KINDS].concat();
    for k in registry::history_sequences(c, &kinds)? {
        validate_properties(&registry::snapshot(c, Some(k))?, &snapshot(c, Some(k))?)?;
    }
    Ok(())
}
