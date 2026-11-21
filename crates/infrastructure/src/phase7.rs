//! Shared revision and treasury guards for metals and company distributions.
use crate::{db, files::*, finance, registry, treasury, Vault};
use patrimoine_domain::*;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub(crate) fn enabled(c: &Connection) -> Result<bool> {
    Ok(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .map_err(db::error)?
            >= 6,
    )
}

pub(crate) fn cutoff(c: &Connection, cutoff: Option<u64>) -> Result<u64> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(invalid("La borne de connaissance demandée n’existe pas."));
    }
    Ok(k)
}

pub(crate) fn expect_sequence(c: &Connection, expected: u64) -> Result<()> {
    if db::sequence(c)? != expected {
        return Err(AppError::new(
            "conflict",
            "Le coffre a changé. Rechargez ses sources avant de les enregistrer ensemble.",
        ));
    }
    Ok(())
}

pub(crate) fn unchanged(
    c: &Connection,
    table: &str,
    column: &str,
    object: Option<&str>,
    value: &str,
) -> Result<()> {
    if let Some(object) = object {
        let old: String = c.query_row(&format!("SELECT v.{column} FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.object_id=?1 ORDER BY cs.sequence DESC LIMIT 1"), [object], |r| r.get(0)).map_err(db::error)?;
        if old != value {
            return Err(invalid(
                "Cette identité conserve sa cible et ses caractéristiques structurelles d’origine.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn immutable_history(c: &Connection, table: &str, columns: &[&str]) -> Result<()> {
    for column in columns {
        let count: u64 = c.query_row(&format!("SELECT count(*) FROM (SELECT r.object_id FROM {table} v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.{column})<>1)"), [], |r| r.get(0)).map_err(db::error)?;
        if count != 0 {
            return Err(invalid("Une identité a changé de cible ou de caractéristiques structurelles entre ses révisions."));
        }
    }
    Ok(())
}

pub(crate) fn preserve_cash_origins(
    c: &Connection,
    old_revisions: &[String],
    new_revisions: &[String],
) -> Result<()> {
    if old_revisions.is_empty() {
        return Ok(());
    }
    let origins = |revisions: &[String]| -> Result<std::collections::BTreeSet<String>> {
        revisions.iter().map(|revision| c.query_row("SELECT r.object_id FROM treasury_transactions t JOIN revisions r ON r.id=t.revision_id WHERE t.revision_id=?1", [revision], |r| r.get(0)).map_err(db::error)).collect()
    };
    if origins(old_revisions)? != origins(new_revisions)? {
        return Err(invalid("Une source déjà rapprochée conserve son opération bancaire d’origine. Corrigez les sources et cette opération ensemble."));
    }
    Ok(())
}

/// A corrected source keeps its original journal operation. Exact revision
/// replacement and current source links are committed by the calling command.
pub(crate) fn resolve_cash(
    c: &Connection,
    transaction: &mut Option<SaveTreasuryTransactionInput>,
    existing: &Option<String>,
    old_revisions: &[String],
    reason: &str,
    shared: (&str, &str),
) -> Result<String> {
    let before = treasury::snapshot(c, None)?;
    match (transaction, existing) {
        (Some(input), None) => {
            treasury::normalize_transaction(input)?;
            if !old_revisions.is_empty() {
                let id = input.id.as_ref().ok_or_else(|| invalid("Cette source possède déjà son opération de trésorerie ; corrigez-la pour conserver une seule origine financière."))?;
                if old_revisions.iter().any(|revision| {
                    !before
                        .transactions
                        .iter()
                        .any(|t| &t.revision_id == revision && &t.id == id)
                }) {
                    return Err(invalid("L’ancienne source financière appartient à une autre opération et ne peut pas être déplacée implicitement."));
                }
            }
            let (_, revision) = registry::begin_shared(
                c,
                input.id.as_deref(),
                input.expected_revision_id.as_deref(),
                "treasury_transaction",
                reason,
                false,
                Some(shared),
            )?;
            treasury::insert_transaction(c, &revision, input)?;
            Ok(revision)
        }
        (None, Some(revision)) => {
            if !before
                .transactions
                .iter()
                .any(|t| &t.revision_id == revision)
            {
                return Err(AppError::new(
                    "conflict",
                    "La révision bancaire choisie n’est plus courante.",
                ));
            }
            if old_revisions.iter().any(|old| old != revision) {
                return Err(invalid(
                    "Réutilisez ou corrigez l’opération bancaire d’origine de cette source.",
                ));
            }
            Ok(revision.clone())
        }
        _ => Err(invalid(
            "Choisissez une opération nouvelle/corrigée ou une révision bancaire existante.",
        )),
    }
}

impl Vault {
    pub(crate) fn write_phase7<T>(
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
        finance::validate_current(&tx)?;
        let result = read(&tx, &id)?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
}
