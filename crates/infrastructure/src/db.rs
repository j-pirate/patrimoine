use crate::files::*;
use patrimoine_application::validate_settings;
use patrimoine_domain::{AppError, DocumentSummary, Result, Settings};
use rusqlite::{params, Connection, OpenFlags};
use std::{collections::BTreeMap, path::Path};

pub(crate) const SCHEMA: u32 = 6;
pub(crate) const APP: &str = env!("CARGO_PKG_VERSION");
pub(crate) const SQL: &str = include_str!("../migrations/0001_foundation.sql");
pub(crate) const SQL2: &str = include_str!("../migrations/0002_registry.sql");
pub(crate) const SQL3: &str = include_str!("../migrations/0003_fx_history.sql");
pub(crate) const SQL4: &str = include_str!("../migrations/0004_property_rentals.sql");
pub(crate) const SQL5: &str = include_str!("../migrations/0005_finance.sql");
pub(crate) const SQL6: &str = include_str!("../migrations/0006_metals_companies.sql");
pub(crate) fn error(e: rusqlite::Error) -> AppError {
    AppError::new(
        "database",
        format!("La base SQLite ne peut pas être utilisée : {e}"),
    )
}
pub(crate) fn connect(path: &Path) -> Result<Connection> {
    no_symlinks(path)?;
    let c = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(error)?;
    c.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(error)?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;").map_err(error)?;
    Ok(c)
}
pub(crate) fn readonly(path: &Path) -> Result<Connection> {
    no_symlinks(path)?;
    let c = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(error)?;
    c.execute_batch("PRAGMA trusted_schema=OFF; PRAGMA foreign_keys=ON; PRAGMA query_only=ON;")
        .map_err(error)?;
    Ok(c)
}
pub(crate) fn create(path: &Path) -> Result<()> {
    new_file(path)?;
    let mut c = connect(path)?;
    let tx = c.transaction().map_err(error)?;
    tx.execute_batch(SQL).map_err(error)?;
    let timestamp = now();
    tx.execute(
        "INSERT INTO schema_migrations VALUES(1,'foundation',?1,?2,?3)",
        params![hash_bytes(SQL.as_bytes()), timestamp, APP],
    )
    .map_err(error)?;
    for (code, name) in [
        ("FR", "France"),
        ("TN", "Tunisie"),
        ("CH", "Suisse"),
        ("AE", "Émirats arabes unis"),
    ] {
        tx.execute("INSERT INTO countries VALUES(?1,?2)", params![code, name])
            .map_err(error)?;
    }
    for (code, name, unit) in [
        ("EUR", "Euro", 2),
        ("TND", "Dinar tunisien", 3),
        ("CHF", "Franc suisse", 2),
        ("AED", "Dirham des Émirats", 2),
        ("USD", "Dollar américain", 2),
    ] {
        tx.execute(
            "INSERT INTO currencies VALUES(?1,?2,?3,1)",
            params![code, name, unit],
        )
        .map_err(error)?;
    }
    let workspace = id();
    tx.execute(
        "INSERT INTO objects VALUES(?1,'workspace',?2)",
        params![workspace, timestamp],
    )
    .map_err(error)?;
    for (key, value) in [
        ("workspace_id", workspace.as_str()),
        ("workspace_name", "Mon patrimoine familial"),
        ("display_currency", "EUR"),
    ] {
        tx.execute(
            "INSERT INTO app_settings VALUES(?1,?2,'text',?3)",
            params![key, value, timestamp],
        )
        .map_err(error)?;
    }
    tx.pragma_update(None, "user_version", 1).map_err(error)?;
    tx.commit().map_err(error)?;
    drop(c);
    migrate_candidate(path)?;
    validate(path)?;
    Ok(())
}
fn schema(c: &Connection) -> Result<Vec<(String, String, String)>> {
    let mut s=c.prepare("SELECT type,name,coalesce(sql,'') FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").map_err(error)?;
    let result = s
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error);
    result
}
pub(crate) fn counts(c: &Connection) -> Result<BTreeMap<String, u64>> {
    let mut out = BTreeMap::new();
    for (kind, table, _) in schema(c)? {
        if kind == "table" {
            let count = c
                .query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |r| {
                    r.get::<_, u64>(0)
                })
                .map_err(error)?;
            out.insert(table, count);
        }
    }
    Ok(out)
}
pub(crate) fn settings(c: &Connection) -> Result<Settings> {
    let get = |key: &str| {
        c.query_row(
            "SELECT value_text FROM app_settings WHERE key=?1",
            [key],
            |r| r.get::<_, String>(0),
        )
        .map_err(error)
    };
    let value = Settings {
        workspace_name: get("workspace_name")?,
        display_currency: get("display_currency")?,
    };
    validate_settings(&value)?;
    Ok(value)
}
pub(crate) fn sequence(c: &Connection) -> Result<u64> {
    c.query_row(
        "SELECT coalesce(max(sequence),0) FROM change_sets",
        [],
        |r| r.get(0),
    )
    .map_err(error)
}
pub(crate) fn mutation(c: &Connection, reason: &str) -> Result<(String, String)> {
    let cs = id();
    let stamp = now();
    c.execute(
        "INSERT INTO change_sets VALUES(?1,?2,?3,?4,?5)",
        params![cs, sequence(c)? + 1, stamp, reason, id()],
    )
    .map_err(error)?;
    Ok((cs, stamp))
}
pub(crate) fn documents(c: &Connection) -> Result<Vec<DocumentSummary>> {
    let mut s=c.prepare("SELECT d.id,v.original_name,v.mime_type,b.size_bytes,b.sha256,r.recorded_at FROM documents d JOIN revisions r ON r.object_id=d.id JOIN document_versions v ON v.revision_id=r.id JOIN document_blobs b ON b.sha256=v.blob_sha256 WHERE r.is_void=0 AND v.status='active' AND NOT EXISTS(SELECT 1 FROM revisions r2 WHERE r2.supersedes_revision_id=r.id) ORDER BY r.recorded_at,d.id").map_err(error)?;
    let result = s
        .query_map([], |r| {
            Ok(DocumentSummary {
                id: r.get(0)?,
                original_name: r.get(1)?,
                mime_type: r.get(2)?,
                size_bytes: r.get(3)?,
                sha256: r.get(4)?,
                added_at: r.get(5)?,
            })
        })
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error);
    result
}
pub(crate) fn blobs(c: &Connection) -> Result<Vec<(String, u64, String)>> {
    let mut s = c
        .prepare("SELECT sha256,size_bytes,relative_path FROM document_blobs ORDER BY sha256")
        .map_err(error)?;
    let result = s
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error);
    result
}
pub(crate) fn validate(path: &Path) -> Result<()> {
    let c = readonly(path)?;
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(error)?;
    if !(1..=SCHEMA).contains(&version) {
        return Err(AppError::new(
            "schema_version",
            format!(
                "Version de base non prise en charge : {version}. Version attendue : {SCHEMA}."
            ),
        ));
    }
    let expected = Connection::open_in_memory().map_err(error)?;
    expected.execute_batch(SQL).map_err(error)?;
    if version >= 2 {
        expected.execute_batch(SQL2).map_err(error)?;
    }
    if version >= 3 {
        expected.execute_batch(SQL3).map_err(error)?;
    }
    if version >= 4 {
        expected.execute_batch(SQL4).map_err(error)?;
    }
    if version >= 5 {
        expected.execute_batch(SQL5).map_err(error)?;
    }
    if version >= 6 {
        expected.execute_batch(SQL6).map_err(error)?;
    }
    if schema(&c)? != schema(&expected)? {
        return Err(invalid(
            "Le schéma SQLite diffère du schéma approuvé de l’application.",
        ));
    }
    let check: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(error)?;
    if check != "ok" {
        return Err(invalid("L’intégrité SQLite est invalide."));
    }
    if c.prepare("PRAGMA foreign_key_check")
        .map_err(error)?
        .exists([])
        .map_err(error)?
    {
        return Err(invalid("Des références SQLite sont incohérentes."));
    }
    let mut migrations = c
        .prepare("SELECT version,checksum FROM schema_migrations ORDER BY version")
        .map_err(error)?;
    let rows = migrations
        .query_map([], |r| Ok((r.get::<_, u32>(0)?, r.get::<_, String>(1)?)))
        .map_err(error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(error)?;
    let expected_migrations: Vec<_> = [SQL, SQL2, SQL3, SQL4, SQL5, SQL6]
        .iter()
        .take(version as usize)
        .enumerate()
        .map(|(i, sql)| ((i + 1) as u32, hash_bytes(sql.as_bytes())))
        .collect();
    if rows != expected_migrations {
        return Err(invalid("Le registre des migrations est invalide."));
    }
    settings(&c)?;
    let workspace: String = c
        .query_row(
            "SELECT value_text FROM app_settings WHERE key='workspace_id'",
            [],
            |r| r.get(0),
        )
        .map_err(error)?;
    if !valid_id(&workspace)
        || c.query_row(
            "SELECT count(*) FROM objects WHERE id=?1 AND kind='workspace'",
            [workspace],
            |r| r.get::<_, u64>(0),
        )
        .map_err(error)?
            != 1
    {
        return Err(invalid("L’identité de l’espace est invalide."));
    }
    for (hash, _, relative) in blobs(&c)? {
        if !valid_hash(&hash) || relative != blob_path(&hash) {
            return Err(invalid("Un chemin documentaire est invalide."));
        }
    }
    let mut ids=c.prepare("SELECT id FROM objects UNION ALL SELECT id FROM revisions UNION ALL SELECT id FROM change_sets UNION ALL SELECT id FROM audit_events").map_err(error)?;
    for value in ids
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(error)?
    {
        if !valid_id(&value.map_err(error)?) {
            return Err(invalid("Un identifiant stable est invalide."));
        }
    }
    for d in documents(&c)? {
        if d.original_name.is_empty()
            || d.original_name.len() > 1024
            || d.original_name.contains(['/', '\\', '\0'])
            || d.mime_type.len() > 200
        {
            return Err(invalid("Une métadonnée documentaire est invalide."));
        }
    }
    let bad:u64=c.query_row("SELECT (SELECT count(*) FROM documents d JOIN objects o ON d.id=o.id WHERE o.kind<>'document') + (SELECT count(*) FROM revisions r JOIN revisions p ON r.supersedes_revision_id=p.id JOIN change_sets rc ON r.change_set_id=rc.id JOIN change_sets pc ON p.change_set_id=pc.id WHERE r.object_id<>p.object_id OR rc.sequence<=pc.sequence) + (SELECT count(*) FROM document_versions v JOIN revisions r ON v.revision_id=r.id WHERE NOT EXISTS(SELECT 1 FROM documents d WHERE d.id=r.object_id))",[],|r|r.get(0)).map_err(error)?;
    if bad != 0 {
        return Err(invalid("Les révisions et types d’objets sont incohérents."));
    }
    let expected_refs = [("countries", 4u64), ("currencies", 5u64)];
    let table_counts = counts(&c)?;
    for (table, count) in expected_refs {
        if table_counts.get(table) != Some(&count) {
            return Err(invalid("Les référentiels du socle sont incomplets."));
        }
    }
    let invalid_refs:u64=c.query_row("SELECT (SELECT count(*) FROM countries WHERE code NOT IN ('FR','TN','CH','AE')) + (SELECT count(*) FROM currencies WHERE code NOT IN ('EUR','TND','CHF','AED','USD') OR active<>1 OR minor_unit<>CASE WHEN code='TND' THEN 3 ELSE 2 END)",[],|r|r.get(0)).map_err(error)?;
    if invalid_refs != 0 {
        return Err(invalid(
            "Les codes pays/devises ou leurs unités sont incompatibles avec le socle.",
        ));
    }
    let incomplete_documents:u64=c.query_row("SELECT count(*) FROM documents d WHERE (SELECT count(*) FROM revisions r JOIN document_versions v ON v.revision_id=r.id WHERE r.object_id=d.id AND NOT EXISTS(SELECT 1 FROM revisions r2 WHERE r2.supersedes_revision_id=r.id))<>1",[],|r|r.get(0)).map_err(error)?;
    if incomplete_documents != 0 {
        return Err(invalid(
            "Un document ne possède pas exactement une version courante.",
        ));
    }
    crate::documents::validate_history(&c)?;
    if version >= 2 {
        crate::registry::validate_history(&c)?;
    }
    // Les modules futurs ne sont pas déclarés validés.
    Ok(())
}
pub(crate) fn validate_generation(generation: &Path) -> Result<()> {
    validate(&generation.join("patrimoine.sqlite3"))?;
    let c = readonly(&generation.join("patrimoine.sqlite3"))?;
    for (hash, size, relative) in blobs(&c)? {
        let (actual, bytes) = hash_file(&generation.join(relative), crate::archives::MAX_FILE)?;
        if actual != hash || bytes != size {
            return Err(invalid(
                "Un original documentaire du coffre est absent ou corrompu.",
            ));
        }
    }
    Ok(())
}

pub(crate) fn version(path: &Path) -> Result<u32> {
    readonly(path)?
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(error)
}
pub(crate) fn copy_snapshot(source: &Path, target: &Path) -> Result<()> {
    new_file(target)?;
    let source = readonly(source)?;
    let mut target = Connection::open(target).map_err(error)?;
    rusqlite::backup::Backup::new(&source, &mut target)
        .map_err(error)?
        .run_to_completion(64, std::time::Duration::from_millis(10), None)
        .map_err(error)?;
    target
        .execute_batch("PRAGMA journal_mode=DELETE; PRAGMA synchronous=FULL;")
        .map_err(error)?;
    Ok(())
}
pub(crate) fn migrate_candidate(path: &Path) -> Result<()> {
    validate(path)?;
    let previous = version(path)?;
    if previous == SCHEMA {
        return Ok(());
    }
    // Uniquement sur une copie candidate : aucune migration en place du coffre utilisateur.
    let mut c = connect(path)?;
    c.execute_batch("PRAGMA foreign_keys=OFF;").map_err(error)?;
    let transaction = c
        .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
        .map_err(error)?;
    for (version, name, sql) in [
        (2, "registry", SQL2),
        (3, "fx_history", SQL3),
        (4, "property_rentals", SQL4),
        (5, "finance", SQL5),
        (6, "metals_companies", SQL6),
    ] {
        if version > previous {
            transaction.execute_batch(sql).map_err(error)?;
            transaction
                .execute(
                    "INSERT INTO schema_migrations VALUES(?1,?2,?3,?4,?5)",
                    params![version, name, hash_bytes(sql.as_bytes()), now(), APP],
                )
                .map_err(error)?;
        }
    }
    transaction
        .pragma_update(None, "user_version", SCHEMA)
        .map_err(error)?;
    if transaction
        .prepare("PRAGMA foreign_key_check")
        .map_err(error)?
        .exists([])
        .map_err(error)?
    {
        return Err(invalid("La migration a produit une référence invalide."));
    }
    transaction.commit().map_err(error)?;
    c.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=DELETE;")
        .map_err(error)?;
    drop(c);
    validate(path)
}
