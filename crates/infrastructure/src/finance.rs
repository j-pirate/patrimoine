//! Cross-module financial validation and explicit scope/claim persistence.
use crate::{
    companies, db, debts, files::*, metals, phase7, properties, registry, rentals, treasury, Vault,
};
use patrimoine_application::FinancialService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, Transaction, TransactionBehavior};

#[cfg(test)]
#[path = "finance_history_root_tests.rs"]
mod history_root_tests;

pub(crate) const TABLES: [(&str, &str); 3] = [
    ("finance_coverage", "finance_coverages"),
    ("rent_claim", "rent_claims"),
    ("rent_claim_adjustment", "rent_claim_adjustments"),
];

pub(crate) fn enabled(c: &Connection) -> Result<bool> {
    Ok(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .map_err(db::error)?
            >= 5,
    )
}

fn claim_parties(c: &Connection, revision: &str) -> rusqlite::Result<Vec<RentClaimParty>> {
    let mut query = c.prepare("SELECT entity_id,economic_share_num,economic_share_den FROM rent_claim_parties WHERE claim_revision_id=?1 ORDER BY entity_id")?;
    let rows = query
        .query_map([revision], |r| {
            let num: Option<String> = r.get(1)?;
            let den: Option<String> = r.get(2)?;
            let economic_share = match (num, den) {
                (Some(num), Some(den)) => Some(Fraction { num, den }),
                (None, None) => None,
                _ => return Err(rusqlite::Error::InvalidQuery),
            };
            Ok(RentClaimParty {
                entity_id: r.get(0)?,
                economic_share,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<FinanceSnapshot> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    if !enabled(c)? {
        return Ok(FinanceSnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    Ok(FinanceSnapshot {
        coverages: registry::load(
            c,
            "finance_coverages",
            "v.valid_from,v.valid_to,v.assets_complete,v.debts_complete,v.rentals_complete,v.notes",
            "",
            k,
            |r| {
                Ok(FinanceCoverage {
                    id: r.get(0)?,
                    revision_id: r.get(1)?,
                    valid_from: r.get(2)?,
                    valid_to: r.get(3)?,
                    assets_complete: r.get(4)?,
                    debts_complete: r.get(5)?,
                    rentals_complete: r.get(6)?,
                    notes: r.get(7)?,
                })
            },
        )?,
        rent_claims: registry::load(
            c,
            "rent_claims",
            "v.due_id,v.valid_from,v.valid_to,v.represented_by_debt_id,v.notes",
            "",
            k,
            |r| {
                let revision_id: String = r.get(1)?;
                Ok(RentClaim {
                    id: r.get(0)?,
                    parties: claim_parties(c, &revision_id)?,
                    revision_id,
                    due_id: r.get(2)?,
                    valid_from: r.get(3)?,
                    valid_to: r.get(4)?,
                    represented_by_debt_id: r.get(5)?,
                    notes: r.get(6)?,
                })
            },
        )?,
        rent_claim_adjustments: registry::load(
            c,
            "rent_claim_adjustments",
            "v.due_id,v.creditor_entity_id,v.kind,v.amount,v.effective_on,v.source,v.notes",
            "",
            k,
            |r| {
                Ok(RentClaimAdjustment {
                    id: r.get(0)?,
                    revision_id: r.get(1)?,
                    due_id: r.get(2)?,
                    creditor_entity_id: r.get(3)?,
                    kind: r.get(4)?,
                    amount: r.get(5)?,
                    effective_on: r.get(6)?,
                    source: r.get(7)?,
                    notes: r.get(8)?,
                })
            },
        )?,
        knowledge_sequence: k,
    })
}

fn validate_at(c: &Connection, cutoff: Option<u64>) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    let registry = registry::snapshot(c, cutoff)?;
    let properties = properties::snapshot(c, cutoff)?;
    let rentals = rentals::snapshot(c, cutoff)?;
    let treasury = treasury::snapshot(c, cutoff)?;
    let debts = debts::snapshot(c, cutoff)?;
    let finance = snapshot(c, cutoff)?;
    validate_treasury(&registry, &properties, &rentals, &treasury)?;
    if cutoff.is_none() {
        debts::validate_current(c)?;
    } else {
        validate_debts(&registry, &debts)?;
    }
    validate_treasury_debt_links(&registry, &treasury, &debts)?;
    validate_finance_sources(
        &registry,
        &properties,
        &rentals,
        &treasury,
        &debts,
        &finance,
    )?;
    if cutoff.is_none() {
        let mismatches = rent_representation_mismatches(
            &registry,
            &properties,
            &rentals,
            &treasury,
            &debts,
            &finance,
        )?;
        if !mismatches.is_empty() {
            // Older releases accepted some mismatches. Preserve their unchanged
            // revisions while allowing independent writes and staged repairs.
            let k = finance
                .knowledge_sequence
                .checked_sub(1)
                .ok_or_else(|| invalid(RENT_REPRESENTATION_ERROR))?;
            let previous_mismatches = rent_representation_mismatches(
                &registry::snapshot(c, Some(k))?,
                &properties::snapshot(c, Some(k))?,
                &rentals::snapshot(c, Some(k))?,
                &treasury::snapshot(c, Some(k))?,
                &debts::snapshot(c, Some(k))?,
                &snapshot(c, Some(k))?,
            )?;
            if !mismatches.is_subset(&previous_mismatches) {
                return Err(invalid(RENT_REPRESENTATION_ERROR));
            }
        }
    }
    if phase7::enabled(c)? {
        if cutoff.is_none() {
            metals::validate_targets(c)?;
            companies::validate_targets(c)?;
        }
        let metals = metals::snapshot(c, cutoff)?;
        let companies = companies::snapshot(c, cutoff)?;
        validate_metals(&registry, &properties, &treasury, &metals)?;
        validate_companies(
            &registry,
            &properties,
            &rentals,
            &treasury,
            &debts,
            &companies,
        )?;
        validate_operation_cash_capacity(&treasury, &metals, &companies)?;
    }
    Ok(())
}

pub(crate) fn validate_current(c: &Connection) -> Result<()> {
    validate_at(c, None)
}

/// Applies to raw histories as well as live snapshots, so a recomputed archive
/// manifest cannot authorize missing child rows, altered tombstones or chains.
pub(crate) fn validate_record_history(c: &Connection, tables: &[(&str, &str)]) -> Result<()> {
    // Keep the root predicate inside COUNT: a nullable UNIQUE index otherwise
    // attracts a global NULL scan for each object on large initial histories.
    for (kind, table) in tables {
        let inconsistent:u64=c.query_row(&format!("SELECT (SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN objects o ON o.id=r.object_id WHERE o.kind<>?1) + (SELECT count(*) FROM revisions r JOIN objects o ON o.id=r.object_id WHERE o.kind=?1 AND NOT EXISTS(SELECT 1 FROM {table} v WHERE v.revision_id=r.id)) + (SELECT count(*) FROM objects o WHERE o.kind=?1 AND ((SELECT count(CASE WHEN r.supersedes_revision_id IS NULL THEN 1 END) FROM revisions r WHERE r.object_id=o.id)<>1 OR (SELECT count(*) FROM revisions r WHERE r.object_id=o.id AND NOT EXISTS(SELECT 1 FROM revisions n WHERE n.supersedes_revision_id=r.id))<>1))"),[kind],|r|r.get(0)).map_err(db::error)?;
        if inconsistent != 0 {
            return Err(invalid(
                "Une source financière possède un type ou une chaîne de révisions incohérente.",
            ));
        }
        let mut columns = c
            .prepare(&format!("PRAGMA table_info({table})"))
            .map_err(db::error)?;
        let columns = columns
            .query_map([], |r| r.get::<_, String>(1))
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        let comparison = columns
            .iter()
            .skip(1)
            .map(|col| format!("p.\"{col}\" IS v.\"{col}\""))
            .collect::<Vec<_>>()
            .join(" AND ");
        let invalid_voids:u64=c.query_row(&format!("SELECT count(*) FROM {table} v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND NOT EXISTS(SELECT 1 FROM {table} p WHERE p.revision_id=r.supersedes_revision_id AND {comparison})"),[],|r|r.get(0)).map_err(db::error)?;
        if invalid_voids != 0 {
            return Err(invalid(
                "Une annulation financière doit conserver le contenu précédent.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    if !enabled(c)? {
        return Ok(());
    }
    validate_record_history(c, &TABLES)?;
    treasury::validate_history(c)?;
    debts::validate_history(c)?;
    metals::validate_history(c)?;
    companies::validate_history(c)?;
    let child_voids:u64=c.query_row("SELECT count(*) FROM rent_claims v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND (EXISTS(SELECT entity_id,economic_share_num,economic_share_den FROM rent_claim_parties WHERE claim_revision_id=r.id EXCEPT SELECT entity_id,economic_share_num,economic_share_den FROM rent_claim_parties WHERE claim_revision_id=r.supersedes_revision_id) OR EXISTS(SELECT entity_id,economic_share_num,economic_share_den FROM rent_claim_parties WHERE claim_revision_id=r.supersedes_revision_id EXCEPT SELECT entity_id,economic_share_num,economic_share_den FROM rent_claim_parties WHERE claim_revision_id=r.id))",[],|r|r.get(0)).map_err(db::error)?;
    if child_voids != 0 {
        return Err(invalid(
            "L’annulation d’une créance doit conserver ses parties.",
        ));
    }
    let changed_due:u64=c.query_row("SELECT count(*) FROM (SELECT r.object_id FROM rent_claims v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.due_id)<>1)",[],|r|r.get(0)).map_err(db::error)?;
    if changed_due != 0 {
        return Err(invalid(
            "Une identité de créance conserve son échéance source.",
        ));
    }
    let mut kinds = [registry::KINDS, properties::KINDS].concat();
    for tables in [
        rentals::TABLES.as_slice(),
        treasury::TABLES.as_slice(),
        debts::TABLES.as_slice(),
        TABLES.as_slice(),
        metals::TABLES.as_slice(),
        companies::TABLES.as_slice(),
    ] {
        kinds.extend(tables.iter().map(|(kind, _)| *kind));
    }
    let sequences = registry::history_sequences(c, &kinds)?;
    for k in sequences {
        validate_at(c, Some(k))?;
    }
    Ok(())
}

impl Vault {
    fn write_finance<T>(
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
        let (id, revision) = registry::begin(&tx, object, expected, kind, reason, false)?;
        write(&tx, &id, &revision)?;
        validate_current(&tx)?;
        let result = read(&tx, &id)?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
}

impl FinancialService for Vault {
    fn list_finance_at(&self, knowledge_sequence: Option<u64>) -> Result<FinanceSnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_finance_coverage(
        &mut self,
        input: SaveFinanceCoverageInput,
    ) -> Result<FinanceCoverage> {
        self.write_finance(
            "finance_coverage",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                tx.execute(
                    "INSERT INTO finance_coverages VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        revision,
                        input.valid_from,
                        input.valid_to,
                        input.assets_complete,
                        input.debts_complete,
                        input.rentals_complete,
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
    fn save_rent_claim(&mut self, mut input: SaveRentClaimInput) -> Result<RentClaim> {
        for party in &mut input.parties {
            if let Some(share) = &mut party.economic_share {
                *share = exact::normalize_fraction(share)?;
            }
        }
        self.write_finance("rent_claim",input.id.as_deref(),input.expected_revision_id.as_deref(),&input.reason,|tx,_,revision|{
            if let Some(object)=&input.id {
                let old:String=tx.query_row("SELECT due_id FROM rent_claims WHERE revision_id=(SELECT supersedes_revision_id FROM revisions WHERE id=?1)",[revision],|r|r.get(0)).map_err(db::error)?;
                if old!=input.due_id {return Err(AppError::new("validation",format!("La créance {object} conserve son échéance d’origine.")));}
            }
            tx.execute("INSERT INTO rent_claims VALUES(?1,?2,?3,?4,?5,?6)",params![revision,input.due_id,input.valid_from,input.valid_to,input.represented_by_debt_id,input.notes]).map_err(db::error)?;
            for party in &input.parties {
                tx.execute("INSERT INTO rent_claim_parties VALUES(?1,?2,?3,?4)",params![revision,party.entity_id,party.economic_share.as_ref().map(|s|s.num.as_str()),party.economic_share.as_ref().map(|s|s.den.as_str())]).map_err(db::error)?;
            }
            Ok(())
        },
            |tx, key| snapshot(tx, None)?
            .rent_claims
            .into_iter()
            .find(|r| r.id == key)
            .ok_or_else(|| invalid("La créance enregistrée est introuvable.")),
        )
    }
    fn save_rent_claim_adjustment(
        &mut self,
        mut input: SaveRentClaimAdjustmentInput,
    ) -> Result<RentClaimAdjustment> {
        input.amount = exact::normalize_decimal_input(&input.amount)?;
        self.write_finance(
            "rent_claim_adjustment",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                tx.execute(
                    "INSERT INTO rent_claim_adjustments VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        revision,
                        input.due_id,
                        input.creditor_entity_id,
                        input.kind,
                        input.amount,
                        input.effective_on,
                        input.source,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .rent_claim_adjustments
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("La dépréciation enregistrée est introuvable."))
            },
        )
    }
    fn get_financial_report(&self, request: FinancialReportRequest) -> Result<FinancialReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let k = request.knowledge_sequence;
        financial_report_with_operations(
            &registry::snapshot(&tx, k)?,
            &properties::snapshot(&tx, k)?,
            &rentals::snapshot(&tx, k)?,
            &treasury::snapshot(&tx, k)?,
            &debts::snapshot(&tx, k)?,
            &snapshot(&tx, k)?,
            &crate::metals::snapshot(&tx, k)?,
            &crate::companies::snapshot(&tx, k)?,
            &request,
        )
    }
}
