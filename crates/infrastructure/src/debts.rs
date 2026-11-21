//! Typed financing persistence. All related rows share the revision transaction.
use crate::{db, files::*, registry, Vault};
use patrimoine_application::DebtService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, Row, Transaction, TransactionBehavior};

pub(crate) const TABLES: [(&str, &str); 9] = [
    ("debt_contract", "debt_contracts"),
    ("debt_terms", "debt_terms"),
    ("debt_movement", "debt_movements"),
    ("debt_receivable_adjustment", "debt_receivable_adjustments"),
    ("debt_schedule", "debt_schedules"),
    ("debt_payment_allocation", "debt_payment_allocations"),
    ("debt_asset_link", "debt_asset_links"),
    ("debt_guarantee", "debt_guarantees"),
    ("debt_coverage", "debt_coverages"),
];

fn fraction(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Fraction>> {
    match (
        row.get::<_, Option<String>>(index)?,
        row.get::<_, Option<String>>(index + 1)?,
    ) {
        (Some(num), Some(den)) => Ok(Some(Fraction { num, den })),
        (None, None) => Ok(None),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}
fn pairs(value: &Option<Fraction>) -> (Option<&str>, Option<&str>) {
    match value {
        Some(v) => (Some(&v.num), Some(&v.den)),
        None => (None, None),
    }
}
fn parties(c: &Connection, revision: &str) -> Result<Vec<FinancingParty>> {
    let mut statement=c.prepare("SELECT line_id,side,entity_id,legal_share_num,legal_share_den,economic_share_num,economic_share_den,liability_kind,notes FROM debt_parties WHERE terms_revision_id=?1 ORDER BY line_id").map_err(db::error)?;
    let result = statement
        .query_map([revision], |r| {
            Ok(FinancingParty {
                line_id: r.get(0)?,
                side: r.get(1)?,
                entity_id: r.get(2)?,
                legal_share: fraction(r, 3)?,
                economic_share: fraction(r, 5)?,
                liability_kind: r.get(7)?,
                notes: r.get(8)?,
            })
        })
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error);
    result
}
fn enabled(c: &Connection) -> Result<bool> {
    Ok(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .map_err(db::error)?
            >= 5,
    )
}

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<DebtSnapshot> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    if !enabled(c)? {
        return Ok(DebtSnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    let mut terms=registry::load(c,"debt_terms","v.debt_id,v.valid_from,v.valid_to,v.principal_status,v.interest_rate,v.interest_convention,v.notes","",k,|r|Ok(DebtTerms {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,valid_from:r.get(3)?,valid_to:r.get(4)?,principal_status:r.get(5)?,interest_rate:r.get(6)?,interest_convention:r.get(7)?,parties:Vec::new(),notes:r.get(8)?}))?;
    for term in &mut terms {
        term.parties = parties(c, &term.revision_id)?;
    }
    Ok(DebtSnapshot {
        contracts:registry::load(c,"debt_contracts","v.name,v.currency,v.tracking_start,v.notes,v.archived","",k,|r|Ok(DebtContract {id:r.get(0)?,revision_id:r.get(1)?,name:r.get(2)?,currency:r.get(3)?,tracking_start:r.get(4)?,notes:r.get(5)?,archived:r.get(6)?}))?,
        terms,
        movements:registry::load(c,"debt_movements","v.debt_id,v.kind,v.effective_on,v.principal_change,v.interest_paid,v.fees_paid,v.unlinked_reason,v.notes","",k,|r|Ok(DebtMovement {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,kind:r.get(3)?,effective_on:r.get(4)?,principal_change:r.get(5)?,interest_paid:r.get(6)?,fees_paid:r.get(7)?,unlinked_reason:r.get(8)?,notes:r.get(9)?}))?,
        receivable_adjustments:registry::load(c,"debt_receivable_adjustments","v.debt_id,v.creditor_entity_id,v.kind,v.effective_on,v.valuation_delta,v.source,v.notes","",k,|r|Ok(DebtReceivableAdjustment {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,creditor_entity_id:r.get(3)?,kind:r.get(4)?,effective_on:r.get(5)?,valuation_delta:r.get(6)?,source:r.get(7)?,notes:r.get(8)?}))?,
        schedules:registry::load(c,"debt_schedules","v.debt_id,v.due_on,v.expected_principal,v.expected_interest,v.expected_fees,v.status,v.notes","",k,|r|Ok(DebtSchedule {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,due_on:r.get(3)?,expected_principal:r.get(4)?,expected_interest:r.get(5)?,expected_fees:r.get(6)?,status:r.get(7)?,notes:r.get(8)?}))?,
        payment_allocations:registry::load(c,"debt_payment_allocations","v.movement_id,v.schedule_id,v.effective_on,v.principal,v.interest,v.fees","",k,|r|Ok(DebtPaymentAllocation {id:r.get(0)?,revision_id:r.get(1)?,movement_id:r.get(2)?,schedule_id:r.get(3)?,effective_on:r.get(4)?,principal:r.get(5)?,interest:r.get(6)?,fees:r.get(7)?}))?,
        asset_links:registry::load(c,"debt_asset_links","v.debt_id,v.asset_id,v.relation_kind,v.attributed_principal_share_num,v.attributed_principal_share_den,v.valid_from,v.valid_to,v.notes","",k,|r|Ok(DebtAssetLink {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,asset_id:r.get(3)?,relation_kind:r.get(4)?,attributed_principal_share:fraction(r,5)?,valid_from:r.get(7)?,valid_to:r.get(8)?,notes:r.get(9)?}))?,
        guarantees:registry::load(c,"debt_guarantees","v.debt_id,v.guarantor_entity_id,v.limit_amount,v.valid_from,v.valid_to,v.notes","",k,|r|Ok(DebtGuarantee {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,guarantor_entity_id:r.get(3)?,limit_amount:r.get(4)?,valid_from:r.get(5)?,valid_to:r.get(6)?,notes:r.get(7)?}))?,
        coverages:registry::load(c,"debt_coverages","v.debt_id,v.company_id,v.side,v.valid_from,v.valid_to,v.included,v.source,v.notes","",k,|r|Ok(DebtCoverage {id:r.get(0)?,revision_id:r.get(1)?,debt_id:r.get(2)?,company_id:r.get(3)?,side:r.get(4)?,valid_from:r.get(5)?,valid_to:r.get(6)?,included:r.get(7)?,source:r.get(8)?,notes:r.get(9)?}))?,
        knowledge_sequence:k,
    })
}

fn unchanged_target(
    c: &Connection,
    table: &str,
    column: &str,
    object: Option<&str>,
    value: &str,
) -> Result<()> {
    if let Some(object) = object {
        let old:String=c.query_row(&format!("SELECT v.{column} FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.object_id=?1 ORDER BY cs.sequence DESC LIMIT 1"),[object],|r|r.get(0)).map_err(db::error)?;
        if old != value {
            return Err(AppError::new("validation","Cette identité de financement conserve sa cible d’origine. Créez un nouvel enregistrement pour changer de cible."));
        }
    }
    Ok(())
}
fn validate_targets(c: &Connection) -> Result<()> {
    for (table, columns) in [
        ("debt_contracts", &["currency", "tracking_start"][..]),
        ("debt_terms", &["debt_id"][..]),
        ("debt_movements", &["debt_id"][..]),
        (
            "debt_receivable_adjustments",
            &["debt_id", "creditor_entity_id"][..],
        ),
        ("debt_schedules", &["debt_id"][..]),
        (
            "debt_payment_allocations",
            &["movement_id", "schedule_id"][..],
        ),
        (
            "debt_asset_links",
            &["debt_id", "asset_id", "relation_kind"][..],
        ),
        ("debt_guarantees", &["debt_id", "guarantor_entity_id"][..]),
        ("debt_coverages", &["debt_id", "company_id", "side"][..]),
    ] {
        for column in columns {
            let count:u64=c.query_row(&format!("SELECT count(*) FROM (SELECT r.object_id FROM {table} v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.{column})<>1)"),[],|r|r.get(0)).map_err(db::error)?;
            if count != 0 {
                return Err(AppError::new("integrity","Une identité de financement a changé de cible ou de devise entre ses révisions."));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_current(c: &Connection) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    validate_targets(c)?;
    validate_debts(&registry::snapshot(c, None)?, &snapshot(c, None)?)
}

pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    validate_targets(c)?;
    for (kind, table) in TABLES {
        let bad:u64=c.query_row(&format!("SELECT (SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN objects o ON o.id=r.object_id WHERE o.kind<>?1) + (SELECT count(*) FROM revisions r JOIN objects o ON o.id=r.object_id WHERE o.kind=?1 AND NOT EXISTS(SELECT 1 FROM {table} v WHERE v.revision_id=r.id)) + (SELECT count(*) FROM objects o WHERE o.kind=?1 AND ((SELECT count(CASE WHEN r.supersedes_revision_id IS NULL THEN 1 END) FROM revisions r WHERE r.object_id=o.id)<>1 OR (SELECT count(*) FROM revisions r WHERE r.object_id=o.id AND NOT EXISTS(SELECT 1 FROM revisions n WHERE n.supersedes_revision_id=r.id))<>1))"),[kind],|r|r.get(0)).map_err(db::error)?;
        if bad != 0 {
            return Err(AppError::new(
                "integrity",
                "Un financement possède un type ou une chaîne de révisions incohérente.",
            ));
        }
        let mut statement = c
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db::error)?;
        let columns = statement
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        let comparisons = columns
            .iter()
            .skip(1)
            .map(|column| format!("p.\"{column}\" IS v.\"{column}\""))
            .collect::<Vec<_>>()
            .join(" AND ");
        let voids:u64=c.query_row(&format!("SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND NOT EXISTS(SELECT 1 FROM {table} p WHERE p.revision_id=r.supersedes_revision_id AND {comparisons})"),[],|r|r.get(0)).map_err(db::error)?;
        if voids != 0 {
            return Err(AppError::new(
                "integrity",
                "Une annulation de financement doit conserver les données précédentes.",
            ));
        }
    }
    let mut statement=c.prepare("SELECT r.id,r.supersedes_revision_id FROM revisions r JOIN objects o ON o.id=r.object_id WHERE o.kind='debt_terms' AND r.is_void=1").map_err(db::error)?;
    let void_terms = statement
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error)?;
    for (revision, previous) in void_terms {
        if parties(c, &revision)? != parties(c, &previous)? {
            return Err(AppError::new(
                "integrity",
                "Une annulation de conditions doit conserver ses parties précédentes.",
            ));
        }
    }
    // Financial history replays validate_debts together with all dependent
    // sources at each relevant K; this function checks raw rows only.
    Ok(())
}

pub(crate) fn normalize_debt_movement(input: &mut SaveDebtMovementInput) -> Result<()> {
    input.principal_change = exact::normalize_decimal_input(&input.principal_change)?;
    input.interest_paid = exact::normalize_decimal_input(&input.interest_paid)?;
    input.fees_paid = exact::normalize_decimal_input(&input.fees_paid)?;
    Ok(())
}
pub(crate) fn insert_debt_movement(
    c: &Connection,
    revision: &str,
    input: &SaveDebtMovementInput,
) -> Result<()> {
    unchanged_target(
        c,
        "debt_movements",
        "debt_id",
        input.id.as_deref(),
        &input.debt_id,
    )?;
    c.execute(
        "INSERT INTO debt_movements VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
        params![
            revision,
            input.debt_id,
            input.kind,
            input.effective_on,
            input.principal_change,
            input.interest_paid,
            input.fees_paid,
            input.unlinked_reason,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

impl Vault {
    fn write_debt<T>(
        &mut self,
        kind: &str,
        object: Option<&str>,
        expected: Option<&str>,
        reason: &str,
        write: impl FnOnce(&Transaction<'_>, &str, &str) -> Result<()>,
        read: impl FnOnce(&Connection, &str) -> Result<T>,
    ) -> Result<T> {
        self.ensure_active()?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let (object, revision) = registry::begin(&tx, object, expected, kind, reason, false)?;
        write(&tx, &object, &revision)?;
        crate::finance::validate_current(&tx)?;
        let result = read(&tx, &object)?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
}

impl DebtService for Vault {
    fn list_debts_at(&self, knowledge_sequence: Option<u64>) -> Result<DebtSnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn get_debt_report(&self, request: DebtReportRequest) -> Result<DebtReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        debt_report(
            &registry::snapshot(&tx, request.knowledge_sequence)?,
            &snapshot(&tx, request.knowledge_sequence)?,
            &request,
        )
    }
    fn save_debt_contract(&mut self, input: SaveDebtContractInput) -> Result<DebtContract> {
        self.write_debt(
            "debt_contract",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_contracts",
                    "currency",
                    input.id.as_deref(),
                    &input.currency,
                )?;
                unchanged_target(
                    tx,
                    "debt_contracts",
                    "tracking_start",
                    input.id.as_deref(),
                    &input.tracking_start,
                )?;
                tx.execute(
                    "INSERT INTO debt_contracts VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        revision,
                        input.name,
                        input.currency,
                        input.tracking_start,
                        input.notes,
                        input.archived
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .contracts
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le contrat enregistré est introuvable."))
            },
        )
    }
    fn save_debt_terms(&mut self, mut input: SaveDebtTermsInput) -> Result<DebtTerms> {
        if let Some(rate) = &mut input.interest_rate {
            *rate = exact::normalize_decimal_input(rate)?;
        }
        for party in &mut input.parties {
            if party.line_id.is_empty() {
                party.line_id = id();
            }
            for share in [&mut party.legal_share, &mut party.economic_share]
                .into_iter()
                .flatten()
            {
                *share = exact::normalize_fraction(share)?;
            }
        }
        self.write_debt(
            "debt_terms",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_terms",
                    "debt_id",
                    input.id.as_deref(),
                    &input.debt_id,
                )?;
                tx.execute(
                    "INSERT INTO debt_terms VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        revision,
                        input.debt_id,
                        input.valid_from,
                        input.valid_to,
                        input.principal_status,
                        input.interest_rate,
                        input.interest_convention,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                for party in &input.parties {
                    let (legal_num, legal_den) = pairs(&party.legal_share);
                    let (economic_num, economic_den) = pairs(&party.economic_share);
                    tx.execute(
                        "INSERT INTO debt_parties VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                        params![
                            revision,
                            party.line_id,
                            party.side,
                            party.entity_id,
                            legal_num,
                            legal_den,
                            economic_num,
                            economic_den,
                            party.liability_kind,
                            party.notes
                        ],
                    )
                    .map_err(db::error)?;
                }
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .terms
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Les conditions enregistrées sont introuvables."))
            },
        )
    }
    fn save_debt_movement(&mut self, mut input: SaveDebtMovementInput) -> Result<DebtMovement> {
        normalize_debt_movement(&mut input)?;
        self.write_debt(
            "debt_movement",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| insert_debt_movement(tx, revision, &input),
            |tx, key| {
                snapshot(tx, None)?
                    .movements
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le mouvement enregistré est introuvable."))
            },
        )
    }
    fn save_debt_receivable_adjustment(
        &mut self,
        mut input: SaveDebtReceivableAdjustmentInput,
    ) -> Result<DebtReceivableAdjustment> {
        input.valuation_delta = exact::normalize_decimal_input(&input.valuation_delta)?;
        self.write_debt(
            "debt_receivable_adjustment",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_receivable_adjustments",
                    "debt_id",
                    input.id.as_deref(),
                    &input.debt_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_receivable_adjustments",
                    "creditor_entity_id",
                    input.id.as_deref(),
                    &input.creditor_entity_id,
                )?;
                tx.execute(
                    "INSERT INTO debt_receivable_adjustments VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        revision,
                        input.debt_id,
                        input.creditor_entity_id,
                        input.kind,
                        input.effective_on,
                        input.valuation_delta,
                        input.source,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .receivable_adjustments
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("L’ajustement de créance enregistré est introuvable."))
            },
        )
    }
    fn save_debt_schedule(&mut self, mut input: SaveDebtScheduleInput) -> Result<DebtSchedule> {
        input.expected_principal = exact::normalize_decimal_input(&input.expected_principal)?;
        input.expected_interest = exact::normalize_decimal_input(&input.expected_interest)?;
        input.expected_fees = exact::normalize_decimal_input(&input.expected_fees)?;
        self.write_debt(
            "debt_schedule",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_schedules",
                    "debt_id",
                    input.id.as_deref(),
                    &input.debt_id,
                )?;
                tx.execute(
                    "INSERT INTO debt_schedules VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        revision,
                        input.debt_id,
                        input.due_on,
                        input.expected_principal,
                        input.expected_interest,
                        input.expected_fees,
                        input.status,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .schedules
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("L’échéance enregistrée est introuvable."))
            },
        )
    }
    fn save_debt_payment_allocation(
        &mut self,
        mut input: SaveDebtPaymentAllocationInput,
    ) -> Result<DebtPaymentAllocation> {
        input.principal = exact::normalize_decimal_input(&input.principal)?;
        input.interest = exact::normalize_decimal_input(&input.interest)?;
        input.fees = exact::normalize_decimal_input(&input.fees)?;
        self.write_debt(
            "debt_payment_allocation",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_payment_allocations",
                    "movement_id",
                    input.id.as_deref(),
                    &input.movement_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_payment_allocations",
                    "schedule_id",
                    input.id.as_deref(),
                    &input.schedule_id,
                )?;
                tx.execute(
                    "INSERT INTO debt_payment_allocations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        revision,
                        input.movement_id,
                        input.schedule_id,
                        input.effective_on,
                        input.principal,
                        input.interest,
                        input.fees
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .payment_allocations
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("L’affectation enregistrée est introuvable."))
            },
        )
    }
    fn save_debt_asset_link(&mut self, mut input: SaveDebtAssetLinkInput) -> Result<DebtAssetLink> {
        if let Some(share) = &mut input.attributed_principal_share {
            *share = exact::normalize_fraction(share)?;
        }
        self.write_debt(
            "debt_asset_link",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_asset_links",
                    "debt_id",
                    input.id.as_deref(),
                    &input.debt_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_asset_links",
                    "asset_id",
                    input.id.as_deref(),
                    &input.asset_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_asset_links",
                    "relation_kind",
                    input.id.as_deref(),
                    &input.relation_kind,
                )?;
                let (num, den) = pairs(&input.attributed_principal_share);
                tx.execute(
                    "INSERT INTO debt_asset_links VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        revision,
                        input.debt_id,
                        input.asset_id,
                        input.relation_kind,
                        num,
                        den,
                        input.valid_from,
                        input.valid_to,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .asset_links
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le lien enregistré est introuvable."))
            },
        )
    }
    fn save_debt_guarantee(&mut self, mut input: SaveDebtGuaranteeInput) -> Result<DebtGuarantee> {
        if let Some(limit) = &mut input.limit_amount {
            *limit = exact::normalize_decimal_input(limit)?;
        }
        self.write_debt(
            "debt_guarantee",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_guarantees",
                    "debt_id",
                    input.id.as_deref(),
                    &input.debt_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_guarantees",
                    "guarantor_entity_id",
                    input.id.as_deref(),
                    &input.guarantor_entity_id,
                )?;
                tx.execute(
                    "INSERT INTO debt_guarantees VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        revision,
                        input.debt_id,
                        input.guarantor_entity_id,
                        input.limit_amount,
                        input.valid_from,
                        input.valid_to,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .guarantees
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("La garantie enregistrée est introuvable."))
            },
        )
    }
    fn save_debt_coverage(&mut self, input: SaveDebtCoverageInput) -> Result<DebtCoverage> {
        self.write_debt(
            "debt_coverage",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                unchanged_target(
                    tx,
                    "debt_coverages",
                    "debt_id",
                    input.id.as_deref(),
                    &input.debt_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_coverages",
                    "company_id",
                    input.id.as_deref(),
                    &input.company_id,
                )?;
                unchanged_target(
                    tx,
                    "debt_coverages",
                    "side",
                    input.id.as_deref(),
                    &input.side,
                )?;
                tx.execute(
                    "INSERT INTO debt_coverages VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        revision,
                        input.debt_id,
                        input.company_id,
                        input.side,
                        input.valid_from,
                        input.valid_to,
                        input.included,
                        input.source,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .coverages
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("La couverture enregistrée est introuvable."))
            },
        )
    }
}
