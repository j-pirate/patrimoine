//! Typed rental persistence. Every mutation is checked before one atomic commit.
use crate::{db, properties, registry, Vault};
use patrimoine_application::RentalService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, Row, Transaction};

const TABLES: [(&str, &str); 6] = [
    ("lease", "leases"),
    ("lease_terms", "lease_terms"),
    ("rent_due", "rent_dues"),
    ("rent_payment", "rent_payments"),
    ("rent_allocation", "rent_allocations"),
    ("rental_expense", "rental_expenses"),
];

fn lease(row: &Row<'_>) -> rusqlite::Result<Lease> {
    Ok(Lease {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        unit_id: row.get(2)?,
        tenant_entity_id: row.get(3)?,
        valid_from: row.get(4)?,
        valid_to: row.get(5)?,
        notes: row.get(6)?,
    })
}

fn terms(row: &Row<'_>) -> rusqlite::Result<LeaseTerms> {
    Ok(LeaseTerms {
        id: row.get(0)?,
        revision_id: row.get(1)?,
        lease_id: row.get(2)?,
        rent: row.get(3)?,
        charges: row.get(4)?,
        currency: row.get(5)?,
        frequency: row.get(6)?,
        due_day: row.get(7)?,
        deposit_required: row.get(8)?,
        valid_from: row.get(9)?,
        valid_to: row.get(10)?,
        notes: row.get(11)?,
    })
}

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<RentalSnapshot> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(db::error)?;
    if version < 4 {
        return Ok(RentalSnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    Ok(RentalSnapshot {
        leases: registry::load(c, "leases", "v.unit_id,v.tenant_entity_id,v.valid_from,v.valid_to,v.notes", "", k, lease)?,
        lease_terms: registry::load(c, "lease_terms", "v.lease_id,v.rent,v.charges,v.currency,v.frequency,v.due_day,v.deposit_required,v.valid_from,v.valid_to,v.notes", "", k, terms)?,
        lease_term_revisions: registry::profile_history(c, "lease_terms", "v.lease_id,v.rent,v.charges,v.currency,v.frequency,v.due_day,v.deposit_required,v.valid_from,v.valid_to,v.notes", "", k, terms)?,
        dues: registry::load(c, "rent_dues", "v.lease_id,v.terms_revision_id,v.period_start,v.period_end,v.due_on,v.rent,v.charges,v.currency,keys.generation_key,v.notes", "JOIN rent_due_keys keys ON keys.due_id=r.object_id", k, |r| Ok(RentDue { id:r.get(0)?, revision_id:r.get(1)?, lease_id:r.get(2)?, terms_revision_id:r.get(3)?, period_start:r.get(4)?, period_end:r.get(5)?, due_on:r.get(6)?, rent:r.get(7)?, charges:r.get(8)?, currency:r.get(9)?, generation_key:r.get(10)?, notes:r.get(11)? }))?,
        payments: registry::load(c, "rent_payments", "v.lease_id,v.kind,v.amount,v.currency,v.effective_on,v.original_payment_id,v.recipient_entity_id,v.reference,v.unlinked_reason", "", k, |r| Ok(RentPayment { id:r.get(0)?,revision_id:r.get(1)?,lease_id:r.get(2)?,kind:r.get(3)?,amount:r.get(4)?,currency:r.get(5)?,effective_on:r.get(6)?,original_payment_id:r.get(7)?,recipient_entity_id:r.get(8)?,reference:r.get(9)?,unlinked_reason:r.get(10)? }))?,
        allocations: registry::load(c, "rent_allocations", "v.payment_id,v.due_id,v.component,v.amount,v.currency,v.effective_on", "", k, |r| Ok(RentAllocation { id:r.get(0)?,revision_id:r.get(1)?,payment_id:r.get(2)?,due_id:r.get(3)?,component:r.get(4)?,amount:r.get(5)?,currency:r.get(6)?,effective_on:r.get(7)? }))?,
        expenses: registry::load(c, "rental_expenses", "v.asset_id,v.kind,v.amount,v.currency,v.effective_on,v.paid_on,v.recoverable,v.notes", "", k, |r| Ok(RentalExpense { id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,kind:r.get(3)?,amount:r.get(4)?,currency:r.get(5)?,effective_on:r.get(6)?,paid_on:r.get(7)?,recoverable:r.get(8)?,notes:r.get(9)? }))?,
        knowledge_sequence: k,
    })
}

/// The typed source must have existed when its due revision was recorded.
fn validate_sources(c: &Connection) -> Result<()> {
    for (table, target) in [
        ("leases", "unit_id"),
        ("lease_terms", "lease_id"),
        ("rent_dues", "lease_id"),
        ("rent_payments", "lease_id"),
        ("rental_expenses", "asset_id"),
    ] {
        let moved: u64 = c.query_row(&format!("SELECT count(*) FROM (SELECT r.object_id FROM {table} v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.{target})<>1)"), [], |r| r.get(0)).map_err(db::error)?;
        if moved != 0 {
            return Err(AppError::new(
                "integrity",
                "Une identité locative ne peut pas changer d’objet cible entre ses révisions.",
            ));
        }
    }
    let changed_currency:u64=c.query_row("SELECT count(*) FROM (SELECT lease_id FROM lease_terms GROUP BY lease_id HAVING count(DISTINCT currency)>1)",[],|r|r.get(0)).map_err(db::error)?;
    if changed_currency != 0 {
        return Err(AppError::new("validation", "Un bail conserve sa devise sur toutes ses conditions et révisions. Créez un nouveau contrat pour changer de devise."));
    }
    let invalid: u64 = c.query_row("SELECT count(*) FROM rent_dues d JOIN revisions dr ON dr.id=d.revision_id JOIN change_sets dc ON dc.id=dr.change_set_id JOIN lease_terms t ON t.revision_id=d.terms_revision_id JOIN revisions tr ON tr.id=t.revision_id JOIN change_sets tc ON tc.id=tr.change_set_id WHERE t.lease_id<>d.lease_id OR tr.is_void<>0 OR tc.sequence>dc.sequence", [], |r| r.get(0)).map_err(db::error)?;
    let invalid_keys: u64 = c.query_row("SELECT (SELECT count(*) FROM rent_due_keys k JOIN objects o ON o.id=k.due_id WHERE o.kind<>'rent_due') + (SELECT count(*) FROM objects o WHERE o.kind='rent_due' AND NOT EXISTS(SELECT 1 FROM rent_due_keys k WHERE k.due_id=o.id))", [], |r| r.get(0)).map_err(db::error)?;
    if invalid != 0 || invalid_keys != 0 {
        return Err(AppError::new("integrity", "Une échéance doit conserver sa clé stable et une révision de conditions connue de son propre bail."));
    }
    Ok(())
}

pub(crate) fn validate_current(c: &Connection) -> Result<()> {
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(db::error)?;
    if version < 4 {
        return Ok(());
    }
    validate_sources(c)?;
    validate_rentals(
        &registry::snapshot(c, None)?,
        &properties::snapshot(c, None)?,
        &snapshot(c, None)?,
    )
}

pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    validate_sources(c)?;
    for (kind, table) in TABLES {
        let bad: u64 = c.query_row(&format!("SELECT (SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN objects o ON o.id=r.object_id WHERE o.kind<>?1) + (SELECT count(*) FROM revisions r JOIN objects o ON o.id=r.object_id WHERE o.kind=?1 AND NOT EXISTS(SELECT 1 FROM {table} v WHERE v.revision_id=r.id)) + (SELECT count(*) FROM objects o WHERE o.kind=?1 AND ((SELECT count(CASE WHEN r.supersedes_revision_id IS NULL THEN 1 END) FROM revisions r WHERE r.object_id=o.id)<>1 OR (SELECT count(*) FROM revisions r WHERE r.object_id=o.id AND NOT EXISTS(SELECT 1 FROM revisions n WHERE n.supersedes_revision_id=r.id))<>1))"), [kind], |r| r.get(0)).map_err(db::error)?;
        if bad != 0 {
            return Err(AppError::new(
                "integrity",
                "Un objet locatif possède un type ou une chaîne de révisions incohérente.",
            ));
        }
        let mut info = c
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db::error)?;
        let columns = info
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        let comparisons = columns
            .into_iter()
            .skip(1)
            .map(|column| format!("p.\"{column}\" IS v.\"{column}\""))
            .collect::<Vec<_>>()
            .join(" AND ");
        let voids:u64 = c.query_row(&format!("SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND NOT EXISTS(SELECT 1 FROM {table} p WHERE p.revision_id=r.supersedes_revision_id AND {comparisons})"), [], |r|r.get(0)).map_err(db::error)?;
        if voids != 0 {
            return Err(AppError::new(
                "integrity",
                "Une annulation locative doit conserver le contenu précédent.",
            ));
        }
    }
    let mut statement = c
        .prepare("SELECT sequence FROM change_sets ORDER BY sequence")
        .map_err(db::error)?;
    let sequences = statement
        .query_map([], |r| r.get::<_, u64>(0))
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error)?;
    for k in sequences {
        validate_rentals(
            &registry::snapshot(c, Some(k))?,
            &properties::snapshot(c, Some(k))?,
            &snapshot(c, Some(k))?,
        )?;
    }
    Ok(())
}

fn unchanged_target(
    c: &Connection,
    table: &str,
    column: &str,
    object: Option<&str>,
    value: &str,
) -> Result<()> {
    if let Some(object) = object {
        let old: String = c.query_row(&format!("SELECT v.{column} FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.object_id=?1 ORDER BY cs.sequence DESC LIMIT 1"), [object], |r|r.get(0)).map_err(db::error)?;
        if old != value {
            return Err(AppError::new("validation", "Cette identité locative conserve son objet d’origine. Créez un nouvel enregistrement pour changer de cible."));
        }
    }
    Ok(())
}

fn insert_due(c: &Connection, revision: &str, input: &GeneratedRentDue) -> Result<()> {
    c.execute(
        "INSERT INTO rent_dues VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            revision,
            input.lease_id,
            input.terms_revision_id,
            input.period_start,
            input.period_end,
            input.due_on,
            input.rent,
            input.charges,
            input.currency,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

pub(crate) fn insert_payment(
    c: &Connection,
    revision: &str,
    input: &SaveRentPaymentInput,
) -> Result<()> {
    c.execute(
        "INSERT INTO rent_payments VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            revision,
            input.lease_id,
            input.kind,
            input.amount,
            input.currency,
            input.effective_on,
            input.original_payment_id,
            input.recipient_entity_id,
            input.reference,
            input.unlinked_reason
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

impl Vault {
    fn write_rental(
        &mut self,
        kind: &str,
        object: Option<&str>,
        expected: Option<&str>,
        reason: &str,
        write: impl FnOnce(&Transaction<'_>, &str, &str) -> Result<()>,
    ) -> Result<String> {
        self.ensure_active()?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let (id, revision) = registry::begin(&tx, object, expected, kind, reason, false)?;
        write(&tx, &id, &revision)?;
        validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        tx.commit().map_err(db::error)?;
        Ok(id)
    }
}

impl RentalService for Vault {
    fn list_rentals_at(&self, knowledge_sequence: Option<u64>) -> Result<RentalSnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_lease(&mut self, input: SaveLeaseInput) -> Result<Lease> {
        let id = self.write_rental(
            "lease",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                unchanged_target(tx, "leases", "unit_id", input.id.as_deref(), &input.unit_id)?;
                tx.execute(
                    "INSERT INTO leases VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        rev,
                        input.unit_id,
                        input.tenant_entity_id,
                        input.valid_from,
                        input.valid_to,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_rentals_at(None)?
            .leases
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::new("integrity", "Le bail enregistré est introuvable."))
    }
    fn save_lease_terms(&mut self, mut input: SaveLeaseTermsInput) -> Result<LeaseTerms> {
        input.rent = exact::normalize_decimal_input(&input.rent)?;
        input.charges = exact::normalize_decimal_input(&input.charges)?;
        input.deposit_required = exact::normalize_decimal_input(&input.deposit_required)?;
        let id = self.write_rental(
            "lease_terms",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                unchanged_target(
                    tx,
                    "lease_terms",
                    "lease_id",
                    input.id.as_deref(),
                    &input.lease_id,
                )?;
                tx.execute(
                    "INSERT INTO lease_terms VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                    params![
                        rev,
                        input.lease_id,
                        input.rent,
                        input.charges,
                        input.currency,
                        input.frequency,
                        input.due_day,
                        input.deposit_required,
                        input.valid_from,
                        input.valid_to,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_rentals_at(None)?
            .lease_terms
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| {
                AppError::new(
                    "integrity",
                    "Les conditions enregistrées sont introuvables.",
                )
            })
    }
    fn save_rent_due(&mut self, mut input: SaveRentDueInput) -> Result<RentDue> {
        input.rent = exact::normalize_decimal_input(&input.rent)?;
        input.charges = exact::normalize_decimal_input(&input.charges)?;
        let draft = GeneratedRentDue {
            lease_id: input.lease_id.clone(),
            terms_revision_id: input.terms_revision_id.clone(),
            period_start: input.period_start.clone(),
            period_end: input.period_end.clone(),
            due_on: input.due_on.clone(),
            rent: input.rent.clone(),
            charges: input.charges.clone(),
            currency: input.currency.clone(),
            generation_key: input.generation_key.clone(),
            notes: input.notes.clone(),
        };
        let id = self.write_rental(
            "rent_due",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, id, rev| {
                unchanged_target(
                    tx,
                    "rent_dues",
                    "lease_id",
                    input.id.as_deref(),
                    &input.lease_id,
                )?;
                if input.id.is_none() {
                    tx.execute(
                        "INSERT INTO rent_due_keys VALUES(?1,?2)",
                        params![id, input.generation_key],
                    )
                    .map_err(db::error)?;
                } else {
                    let key: String = tx
                        .query_row(
                            "SELECT generation_key FROM rent_due_keys WHERE due_id=?1",
                            [id],
                            |r| r.get(0),
                        )
                        .map_err(db::error)?;
                    if key != input.generation_key {
                        return Err(AppError::new(
                            "validation",
                            "La clé stable d’une échéance ne peut pas être modifiée.",
                        ));
                    }
                }
                insert_due(tx, rev, &draft)
            },
        )?;
        self.list_rentals_at(None)?
            .dues
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::new("integrity", "L’échéance enregistrée est introuvable."))
    }
    fn save_rent_payment(&mut self, mut input: SaveRentPaymentInput) -> Result<RentPayment> {
        input.amount = exact::normalize_decimal_input(&input.amount)?;
        let id = self.write_rental(
            "rent_payment",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                unchanged_target(
                    tx,
                    "rent_payments",
                    "lease_id",
                    input.id.as_deref(),
                    &input.lease_id,
                )?;
                insert_payment(tx, rev, &input)
            },
        )?;
        self.list_rentals_at(None)?
            .payments
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::new("integrity", "Le paiement enregistré est introuvable."))
    }
    fn refund_rent_payment(&mut self, mut input: RefundRentPaymentInput) -> Result<RentPayment> {
        self.ensure_active()?;
        if input.payment.kind != "refund" {
            return Err(AppError::new(
                "validation",
                "Cette commande enregistre un remboursement lié à un encaissement d’origine.",
            ));
        }
        input.payment.amount = exact::normalize_decimal_input(&input.payment.amount)?;
        for allocation in &mut input.allocations {
            allocation.amount = exact::normalize_decimal_input(&allocation.amount)?;
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let before = snapshot(&tx, None)?;
        if before.knowledge_sequence != input.expected_sequence {
            return Err(AppError::new("conflict", "Le registre locatif a changé. Rechargez avant de remplacer les affectations du remboursement."));
        }
        if let Some(id) = &input.payment.id {
            let previous = before
                .payments
                .iter()
                .find(|p| &p.id == id)
                .ok_or_else(|| {
                    AppError::new("not_found", "Le remboursement à corriger n’existe plus.")
                })?;
            if previous.kind != "refund" || previous.lease_id != input.payment.lease_id {
                return Err(AppError::new(
                    "validation",
                    "La correction conserve l’identité du remboursement et son bail d’origine.",
                ));
            }
        }
        let (change, stamp) = db::mutation(&tx, input.payment.reason.trim())?;
        let (payment_id, revision) = registry::begin_shared(
            &tx,
            input.payment.id.as_deref(),
            input.payment.expected_revision_id.as_deref(),
            "rent_payment",
            &input.payment.reason,
            false,
            Some((&change, &stamp)),
        )?;
        insert_payment(&tx, &revision, &input.payment)?;
        for previous in before
            .allocations
            .iter()
            .filter(|a| a.payment_id == payment_id)
        {
            let (_, revision) = registry::begin_shared(
                &tx,
                Some(&previous.id),
                Some(&previous.revision_id),
                "rent_allocation",
                &input.payment.reason,
                true,
                Some((&change, &stamp)),
            )?;
            tx.execute(
                "INSERT INTO rent_allocations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    revision,
                    previous.payment_id,
                    previous.due_id,
                    previous.component,
                    previous.amount,
                    previous.currency,
                    previous.effective_on
                ],
            )
            .map_err(db::error)?;
        }
        for allocation in input.allocations {
            let (_, revision) = registry::begin_shared(
                &tx,
                None,
                None,
                "rent_allocation",
                &input.payment.reason,
                false,
                Some((&change, &stamp)),
            )?;
            tx.execute(
                "INSERT INTO rent_allocations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    revision,
                    payment_id,
                    allocation.due_id,
                    allocation.component,
                    allocation.amount,
                    input.payment.currency,
                    input.payment.effective_on
                ],
            )
            .map_err(db::error)?;
        }
        validate_current(&tx)?;
        crate::finance::validate_current(&tx)?;
        let result = snapshot(&tx, None)?
            .payments
            .into_iter()
            .find(|p| p.id == payment_id)
            .ok_or_else(|| {
                AppError::new("integrity", "Le remboursement enregistré est introuvable.")
            })?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn save_rent_allocation(
        &mut self,
        mut input: SaveRentAllocationInput,
    ) -> Result<RentAllocation> {
        input.amount = exact::normalize_decimal_input(&input.amount)?;
        let id = self.write_rental(
            "rent_allocation",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                tx.execute(
                    "INSERT INTO rent_allocations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        rev,
                        input.payment_id,
                        input.due_id,
                        input.component,
                        input.amount,
                        input.currency,
                        input.effective_on
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_rentals_at(None)?
            .allocations
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::new("integrity", "L’affectation enregistrée est introuvable."))
    }
    fn save_rental_expense(&mut self, mut input: SaveRentalExpenseInput) -> Result<RentalExpense> {
        input.amount = exact::normalize_decimal_input(&input.amount)?;
        let id = self.write_rental(
            "rental_expense",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                unchanged_target(
                    tx,
                    "rental_expenses",
                    "asset_id",
                    input.id.as_deref(),
                    &input.asset_id,
                )?;
                tx.execute(
                    "INSERT INTO rental_expenses VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        rev,
                        input.asset_id,
                        input.kind,
                        input.amount,
                        input.currency,
                        input.effective_on,
                        input.paid_on,
                        input.recoverable,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_rentals_at(None)?
            .expenses
            .into_iter()
            .find(|r| r.id == id)
            .ok_or_else(|| AppError::new("integrity", "La dépense enregistrée est introuvable."))
    }
    fn generate_rent_dues(&mut self, input: GenerateRentDuesInput) -> Result<RentGenerationResult> {
        self.ensure_active()?;
        if input.reason.trim().is_empty() || input.reason.len() > 2000 {
            return Err(AppError::new(
                "validation",
                "Indiquez un motif de génération entre 1 et 2 000 octets.",
            ));
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let before = snapshot(&tx, None)?;
        if input.expected_sequence != before.knowledge_sequence {
            return Err(AppError::new(
                "conflict",
                "Le registre a changé. Rechargez avant de générer les échéances.",
            ));
        }
        let drafts = patrimoine_domain::generate_rent_dues(
            &registry::snapshot(&tx, None)?,
            &properties::snapshot(&tx, None)?,
            &before,
            &input,
        )?;
        let mut existing_count = 0;
        let mut missing = Vec::new();
        for draft in drafts {
            let previous = before
                .dues
                .iter()
                .find(|d| d.generation_key == draft.generation_key);
            if let Some(row) = previous {
                if row.lease_id != draft.lease_id
                    || row.terms_revision_id != draft.terms_revision_id
                    || row.period_start != draft.period_start
                    || row.period_end != draft.period_end
                    || row.due_on != draft.due_on
                    || row.rent != draft.rent
                    || row.charges != draft.charges
                    || row.currency != draft.currency
                {
                    return Err(AppError::new("conflict","Une échéance existante diffère des conditions actuelles. Corrigez-la explicitement ou saisissez une régularisation ; ses paiements n’ont pas été réécrits."));
                }
                existing_count += 1;
            } else {
                let reserved: bool = tx
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM rent_due_keys WHERE generation_key=?1)",
                        [&draft.generation_key],
                        |r| r.get(0),
                    )
                    .map_err(db::error)?;
                if reserved {
                    return Err(AppError::new("conflict","La clé de cette échéance a déjà été utilisée puis annulée. Son historique doit être corrigé explicitement."));
                }
                missing.push(draft);
            }
        }
        let mut ids = Vec::new();
        if !missing.is_empty() {
            let (change, stamp) = db::mutation(&tx, input.reason.trim())?;
            for draft in missing {
                let (id, revision) = registry::begin_shared(
                    &tx,
                    None,
                    None,
                    "rent_due",
                    &input.reason,
                    false,
                    Some((&change, &stamp)),
                )?;
                tx.execute(
                    "INSERT INTO rent_due_keys VALUES(?1,?2)",
                    params![id, draft.generation_key],
                )
                .map_err(db::error)?;
                insert_due(&tx, &revision, &draft)?;
                ids.push(id);
            }
            validate_current(&tx)?;
            crate::finance::validate_current(&tx)?;
        }
        let after = snapshot(&tx, None)?;
        let created = after
            .dues
            .into_iter()
            .filter(|d| ids.contains(&d.id))
            .collect();
        let knowledge_sequence = after.knowledge_sequence;
        tx.commit().map_err(db::error)?;
        Ok(RentGenerationResult {
            created,
            existing_count,
            knowledge_sequence,
        })
    }
    fn get_rental_report(&self, request: RentalReportRequest) -> Result<RentalReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let k = request.knowledge_sequence;
        patrimoine_domain::financial_rental_report(
            &registry::snapshot(&tx, k)?,
            &properties::snapshot(&tx, k)?,
            &snapshot(&tx, k)?,
            &crate::treasury::snapshot(&tx, k)?,
            &crate::debts::snapshot(&tx, k)?,
            &crate::finance::snapshot(&tx, k)?,
            &request,
        )
    }
}
