use patrimoine_application::{
    DebtService, FinancialService, PropertyService, RegistryService, RentalService,
    TreasuryService, VaultService,
};
use patrimoine_infrastructure::Vault;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use uuid::Uuid;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

const FOUNDATION: &str = include_str!("../migrations/0001_foundation.sql");
const REGISTRY: &str = include_str!("../migrations/0002_registry.sql");
const FX_HISTORY: &str = include_str!("../migrations/0003_fx_history.sql");
const PROPERTY_RENTALS: &str = include_str!("../migrations/0004_property_rentals.sql");
const FINANCE: &str = include_str!("../migrations/0005_finance.sql");
const STAMP: &str = "2026-01-02T03:04:05Z";
fn id() -> String {
    Uuid::new_v4().to_string()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn db_path(root: &Path) -> PathBuf {
    root.join("generations")
        .join(fs::read_to_string(root.join("CURRENT")).unwrap().trim())
        .join("patrimoine.sqlite3")
}
struct Legacy {
    root: PathBuf,
    generation: String,
    workspace: String,
    document: String,
    revision: String,
    bytes: Vec<u8>,
}
impl Legacy {
    fn path(&self) -> PathBuf {
        self.root.join("generations").join(&self.generation)
    }
    fn db(&self) -> PathBuf {
        self.path().join("patrimoine.sqlite3")
    }
}
// A genuine approved 0001 database, rather than a schema-2 database relabelled 1.
fn legacy(root: &Path) -> Legacy {
    let value = Legacy {
        root: root.to_owned(),
        generation: id(),
        workspace: id(),
        document: id(),
        revision: id(),
        bytes: b"%PDF-1.7\nOriginal familial non modifie\n".to_vec(),
    };
    fs::create_dir_all(value.path()).unwrap();
    fs::write(root.join("CURRENT"), format!("{}\n", value.generation)).unwrap();
    let mut c = Connection::open(value.db()).unwrap();
    let tx = c.transaction().unwrap();
    tx.execute_batch(FOUNDATION).unwrap();
    tx.execute(
        "INSERT INTO schema_migrations VALUES(1,'foundation',?1,?2,'0.1.0')",
        params![hash(FOUNDATION.as_bytes()), STAMP],
    )
    .unwrap();
    for (code, name) in [
        ("FR", "France"),
        ("TN", "Tunisie"),
        ("CH", "Suisse"),
        ("AE", "Emirats"),
    ] {
        tx.execute("INSERT INTO countries VALUES(?1,?2)", params![code, name])
            .unwrap();
    }
    for (code, unit) in [("EUR", 2), ("TND", 3), ("CHF", 2), ("AED", 2), ("USD", 2)] {
        tx.execute(
            "INSERT INTO currencies VALUES(?1,?1,?2,1)",
            params![code, unit],
        )
        .unwrap();
    }
    tx.execute(
        "INSERT INTO objects VALUES(?1,'workspace',?2)",
        params![value.workspace, STAMP],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO objects VALUES(?1,'document',?2)",
        params![value.document, STAMP],
    )
    .unwrap();
    for (key, text) in [
        ("workspace_id", value.workspace.as_str()),
        ("workspace_name", "Famille historique"),
        ("display_currency", "TND"),
    ] {
        tx.execute(
            "INSERT INTO app_settings VALUES(?1,?2,'text',?3)",
            params![key, text, STAMP],
        )
        .unwrap();
    }
    let change = id();
    tx.execute(
        "INSERT INTO change_sets VALUES(?1,1,?2,'Import historique',?3)",
        params![change, STAMP, id()],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO revisions VALUES(?1,?2,?3,?4,NULL,0,'Import historique')",
        params![value.revision, value.document, change, STAMP],
    )
    .unwrap();
    tx.execute(
        "INSERT INTO audit_events VALUES(?1,?2,?3,?4,'document_imported','Original historique')",
        params![id(), change, value.document, value.revision],
    )
    .unwrap();
    let digest = hash(&value.bytes);
    let relative = format!("documents/sha256/{}/{digest}", &digest[..2]);
    tx.execute(
        "INSERT INTO document_blobs VALUES(?1,?2,?3,?4)",
        params![digest, value.bytes.len(), relative, STAMP],
    )
    .unwrap();
    tx.execute("INSERT INTO documents VALUES(?1)", [&value.document])
        .unwrap();
    tx.execute("INSERT INTO document_versions VALUES(?1,?2,'acte familial.pdf','application/pdf','Ancien original','active')",params![value.revision,digest]).unwrap();
    tx.pragma_update(None, "user_version", 1).unwrap();
    tx.commit().unwrap();
    drop(c);
    fs::create_dir_all(value.path().join(&relative).parent().unwrap()).unwrap();
    fs::write(value.path().join(relative), &value.bytes).unwrap();
    value
}
struct LegacyRegistry {
    vault: Legacy,
    person: String,
    initial_profile: String,
    corrected_profile: String,
    membership: String,
    asset: String,
}
fn legacy_revision(c: &Connection, object: &str, kind: &str, previous: Option<&str>) -> String {
    if previous.is_none() {
        c.execute(
            "INSERT INTO objects VALUES(?1,?2,?3)",
            params![object, kind, STAMP],
        )
        .unwrap();
    }
    let sequence: u64 = c
        .query_row("SELECT max(sequence)+1 FROM change_sets", [], |r| r.get(0))
        .unwrap();
    let change = id();
    let revision = id();
    c.execute(
        "INSERT INTO change_sets VALUES(?1,?2,?3,'Historique approuvé',?4)",
        params![change, sequence, STAMP, id()],
    )
    .unwrap();
    c.execute(
        "INSERT INTO revisions VALUES(?1,?2,?3,?4,?5,0,'Historique approuvé')",
        params![revision, object, change, STAMP, previous],
    )
    .unwrap();
    c.execute(
        "INSERT INTO audit_events VALUES(?1,?2,?3,?4,'registry_saved','Historique approuvé')",
        params![id(), change, object, revision],
    )
    .unwrap();
    revision
}
// Build the exact approved 0001 + 0002 schema, with a document and civil history.
// No database produced by the current application is relabelled as an old version.
fn legacy_registry(root: &Path) -> LegacyRegistry {
    let vault = legacy(root);
    let c = Connection::open(vault.db()).unwrap();
    // The approved 0002 script rebuilds `objects`; as in the original migrator,
    // suspend FK enforcement only for that rebuild, then check every reference.
    c.pragma_update(None, "foreign_keys", false).unwrap();
    c.execute_batch(REGISTRY).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    assert!(!c
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    c.execute(
        "INSERT INTO schema_migrations VALUES(2,'registry',?1,?2,'0.1.0')",
        params![hash(REGISTRY.as_bytes()), STAMP],
    )
    .unwrap();
    let person = id();
    let initial_profile = legacy_revision(&c, &person, "entity", None);
    c.execute("INSERT INTO entities VALUES(?1,'person')", [&person])
        .unwrap();
    c.execute("INSERT INTO entity_profiles VALUES(?1,?2,'Ada Martin','Ada','Martin',NULL,'FR','EUR','','Profil initial',0)",params![initial_profile,person]).unwrap();
    let corrected_profile = legacy_revision(&c, &person, "entity", Some(&initial_profile));
    c.execute("INSERT INTO entity_profiles VALUES(?1,?2,'Ada Martin corrigée','Ada','Martin',NULL,'TN','EUR','','Correction civile',0)",params![corrected_profile,person]).unwrap();
    let membership = id();
    let revision = legacy_revision(&c, &membership, "membership", None);
    c.execute(
        "INSERT INTO family_memberships VALUES(?1,?2,'2020-01-01',NULL,'parent')",
        params![revision, person],
    )
    .unwrap();
    let asset = id();
    let revision = legacy_revision(&c, &asset, "asset", None);
    c.execute("INSERT INTO assets VALUES(?1)", [&asset])
        .unwrap();
    c.execute(
        "INSERT INTO asset_profiles VALUES(?1,?2,'Maison historique','FR','real_estate','',0)",
        params![revision, asset],
    )
    .unwrap();
    let ownership = id();
    let revision = legacy_revision(&c, &ownership, "ownership", None);
    c.execute(
        "INSERT INTO asset_ownerships VALUES(?1,?2,?3,'1','1','1','1','2020-01-01',NULL)",
        params![revision, asset, person],
    )
    .unwrap();
    let valuation = id();
    let revision = legacy_revision(&c, &valuation, "valuation", None);
    c.execute(
        "INSERT INTO valuations VALUES(?1,'asset',?2,'100000','EUR','2025-01-01','Acte conservé')",
        params![revision, asset],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 2).unwrap();
    LegacyRegistry {
        vault,
        person,
        initial_profile,
        corrected_profile,
        membership,
        asset,
    }
}
// An approved schema 3 is constructed from the historical SQL scripts,
// never by changing user_version on a database produced by the new application.
fn legacy_fx(root: &Path) -> LegacyRegistry {
    let old = legacy_registry(root);
    let c = Connection::open(old.vault.db()).unwrap();
    c.pragma_update(None, "foreign_keys", false).unwrap();
    c.execute_batch(FX_HISTORY).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(3,'fx_history',?1,?2,'0.1.0')",
        params![hash(FX_HISTORY.as_bytes()), STAMP],
    )
    .unwrap();
    let rate = id();
    let revision = legacy_revision(&c, &rate, "exchange_rate", None);
    c.execute("INSERT INTO exchange_rates VALUES(?1,'TND','EUR','0.3','2025-01-01','Cours historique','')",[revision]).unwrap();
    let period = id();
    let revision = legacy_revision(&c, &period, "profile_period", None);
    c.execute(
        "INSERT INTO profile_periods VALUES(?1,'entity',?2,?3,'2020-01-01',NULL)",
        params![revision, old.person, old.initial_profile],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 3).unwrap();
    assert!(!c
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    old
}

fn assert_registry_history(vault: &Vault, old: &LegacyRegistry) {
    let latest = vault.list_registry().unwrap();
    assert_eq!(latest.knowledge_sequence, 7);
    assert_eq!(latest.entities[0].id, old.person);
    assert_eq!(latest.entities[0].revision_id, old.corrected_profile);
    assert_eq!(latest.memberships[0].id, old.membership);
    assert_eq!(latest.assets[0].id, old.asset);
    assert!(latest.exchange_rates.is_empty());
    assert!(latest.profile_periods.is_empty());
    assert_eq!(latest.profile_revisions.len(), 2);
    let before = vault.list_registry_at(Some(2)).unwrap();
    assert_eq!(before.entities[0].revision_id, old.initial_profile);
    assert_eq!(before.profile_revisions.len(), 1);
    assert_eq!(
        before.profile_revisions[0].residence_country.as_deref(),
        Some("FR")
    );
    let corrected = vault.list_registry_at(Some(3)).unwrap();
    assert_eq!(corrected.entities[0].revision_id, old.corrected_profile);
    assert_eq!(corrected.profile_revisions.len(), 2);
    // All timestamps are deliberately equal: only sequence determines knowledge.
    assert_eq!(
        vault
            .list_registry_history()
            .unwrap()
            .events
            .iter()
            .map(|v| v.sequence)
            .collect::<Vec<_>>(),
        vec![7, 6, 5, 4, 3, 2, 1]
    );
}
fn tables(c: &Connection) -> BTreeMap<String, u64> {
    let mut query = c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap();
    query
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|name| {
            let name = name.unwrap();
            let count = c
                .query_row(&format!("SELECT count(*) FROM \"{name}\""), [], |r| {
                    r.get(0)
                })
                .unwrap();
            (name, count)
        })
        .collect()
}
fn schema(path: &Path) -> Vec<(String, String, String)> {
    let c = Connection::open(path).unwrap();
    let mut q = c.prepare("SELECT type,name,coalesce(sql,'') FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").unwrap();
    q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .map(|r| r.unwrap())
        .collect()
}
fn archive(value: &Legacy, target: &Path) {
    let c = Connection::open(value.db()).unwrap();
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap();
    let sequence: u64 = c
        .query_row("SELECT max(sequence) FROM change_sets", [], |r| r.get(0))
        .unwrap();
    let digest = hash(&value.bytes);
    let files = vec![
        (
            "data/patrimoine.sqlite3".to_owned(),
            fs::read(value.db()).unwrap(),
        ),
        (
            format!("documents/sha256/{}/{digest}", &digest[..2]),
            value.bytes.clone(),
        ),
        (
            "dictionary/schema.json".to_owned(),
            serde_json::to_vec(&json!({"schema_version":version})).unwrap(),
        ),
        (
            "recovery/LISEZMOI.md".to_owned(),
            b"Recuperation historique SQLite".to_vec(),
        ),
    ];
    let manifest = json!({"archive_kind":"full_backup","format_version":1,"schema_version":version,"app_version":"0.1.0","export_id":id(),"created_at":STAMP,"generation_id":value.generation,"mutation_sequence":sequence,"document_count":1,"blob_count":1,"table_counts":tables(&c),"files":files.iter().map(|(name,bytes)|json!({"path":name,"size_bytes":bytes.len(),"sha256":hash(bytes)})).collect::<Vec<Value>>()});
    let mut zip = ZipWriter::new(fs::File::create(target).unwrap());
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(0o600);
    for (name, bytes) in files {
        zip.start_file(name, options).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.start_file("manifest.json", options).unwrap();
    zip.write_all(&serde_json::to_vec(&manifest).unwrap())
        .unwrap();
    zip.finish().unwrap();
}
fn assert_original(vault: &Vault, legacy: &Legacy, export: &Path) {
    let documents = vault.list_documents().unwrap();
    assert_eq!(documents.len(), 1);
    assert_eq!(documents[0].id, legacy.document);
    assert_eq!(documents[0].sha256, hash(&legacy.bytes));
    vault.export_document(&legacy.document, export).unwrap();
    assert_eq!(fs::read(export).unwrap(), legacy.bytes);
    let c = Connection::open(db_path(&legacy.root)).unwrap();
    let preserved: String = c
        .query_row(
            "SELECT id FROM revisions WHERE object_id=?1",
            [&legacy.document],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(preserved, legacy.revision);
    let workspace: String = c
        .query_row(
            "SELECT value_text FROM app_settings WHERE key='workspace_id'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(workspace, legacy.workspace);
}

#[test]
fn approved_legacy_open_creates_new_generation_and_retains_original_bytes_and_identity() {
    let temp = TempDir::new().unwrap();
    let old = legacy(&temp.path().join("legacy"));
    let original = fs::read(old.db()).unwrap();
    let mut vault = Vault::open(&old.root).unwrap();
    let state = vault.status().unwrap();
    assert_eq!(state.schema_version, 6);
    assert_ne!(state.generation_id, old.generation);
    assert_eq!(state.mutation_sequence, 1);
    assert_eq!(state.settings.workspace_name, "Famille historique");
    assert_eq!(fs::read(old.db()).unwrap(), original);
    assert_original(&vault, &old, &temp.path().join("opened.pdf"));
    assert!(vault.list_registry().unwrap().entities.is_empty());
    let fresh_root = temp.path().join("fresh");
    let _fresh = Vault::open(&fresh_root).unwrap();
    assert_eq!(schema(&db_path(&old.root)), schema(&db_path(&fresh_root)));
    let backup = temp.path().join("migrated.zip");
    vault.create_backup(&backup).unwrap();
    drop(vault);
    assert_eq!(
        Vault::open(&old.root)
            .unwrap()
            .status()
            .unwrap()
            .generation_id,
        state.generation_id
    );
}

#[test]
fn schema_two_open_preserves_document_family_and_tied_timestamp_history_without_inventing_periods()
{
    let temp = TempDir::new().unwrap();
    let old = legacy_registry(&temp.path().join("legacy-two"));
    let original = fs::read(old.vault.db()).unwrap();
    let vault = Vault::open(&old.vault.root).unwrap();
    let status = vault.status().unwrap();
    assert_eq!(status.schema_version, 6);
    assert_ne!(status.generation_id, old.vault.generation);
    assert_eq!(fs::read(old.vault.db()).unwrap(), original);
    assert_original(&vault, &old.vault, &temp.path().join("preserved.pdf"));
    assert_registry_history(&vault, &old);
    let fresh_root = temp.path().join("fresh-three");
    let _fresh = Vault::open(&fresh_root).unwrap();
    assert_eq!(
        schema(&db_path(&old.vault.root)),
        schema(&db_path(&fresh_root))
    );
}

#[test]
fn schema_two_archive_is_validated_then_migrated_with_original_archive_and_generations_retained() {
    let temp = TempDir::new().unwrap();
    let old = legacy_registry(&temp.path().join("legacy-two"));
    let zip = temp.path().join("legacy-two.zip");
    archive(&old.vault, &zip);
    let original = fs::read(&zip).unwrap();
    let root = temp.path().join("active");
    let mut vault = Vault::open(&root).unwrap();
    let prior = vault.status().unwrap().generation_id;
    let prior_sequence = vault.status().unwrap().mutation_sequence;
    let inspection = vault.inspect_restore(&zip).unwrap();
    assert_eq!(inspection.schema_version, 2);
    // The inspection sequence binds its token to the active vault, so that a
    // later local edit invalidates activation; it is not the source sequence.
    assert_eq!(inspection.mutation_sequence, prior_sequence);
    let restored = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(restored.schema_version, 6);
    assert_eq!(restored.mutation_sequence, 7);
    assert!(root.join("generations").join(prior).is_dir());
    assert_eq!(fs::read(&zip).unwrap(), original);
    assert_registry_history(&vault, &old);
    vault
        .export_document(&old.vault.document, &temp.path().join("restored-two.pdf"))
        .unwrap();
    assert_eq!(
        fs::read(temp.path().join("restored-two.pdf")).unwrap(),
        old.vault.bytes
    );
    let backup = temp.path().join("roundtrip-three.zip");
    vault.create_backup(&backup).unwrap();
    let next = vault.inspect_restore(&backup).unwrap();
    assert_eq!(next.schema_version, 6);
    vault.activate_restore(&next.token, true, None).unwrap();
    assert_registry_history(&vault, &old);
}

#[test]
fn legacy_zip_is_validated_before_candidate_migration_and_roundtrips_current_schema() {
    let temp = TempDir::new().unwrap();
    let old = legacy(&temp.path().join("legacy"));
    let zip = temp.path().join("old.zip");
    archive(&old, &zip);
    let archive_hash = hash(&fs::read(&zip).unwrap());
    let root = temp.path().join("active");
    let mut vault = Vault::open(&root).unwrap();
    let active = vault.status().unwrap();
    let inspection = vault.inspect_restore(&zip).unwrap();
    assert_eq!(inspection.schema_version, 1);
    assert_eq!(inspection.source_generation_id, old.generation);
    assert_eq!(vault.status().unwrap().generation_id, active.generation_id);
    let candidate = Connection::open(
        root.join("staging")
            .join(&inspection.token)
            .join("data/patrimoine.sqlite3"),
    )
    .unwrap();
    assert_eq!(
        candidate
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        6
    );
    drop(candidate);
    let restored = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(restored.schema_version, 6);
    assert_eq!(restored.settings.display_currency, "TND");
    assert_eq!(vault.list_documents().unwrap()[0].id, old.document);
    assert_eq!(hash(&fs::read(&zip).unwrap()), archive_hash);
    assert!(root.join("generations").join(active.generation_id).exists());
    let roundtrip = temp.path().join("new.zip");
    vault.create_backup(&roundtrip).unwrap();
    let next = vault.inspect_restore(&roundtrip).unwrap();
    assert_eq!(next.schema_version, 6);
    vault.activate_restore(&next.token, true, None).unwrap();
    vault
        .export_document(&old.document, &temp.path().join("restored.pdf"))
        .unwrap();
    assert_eq!(
        fs::read(temp.path().join("restored.pdf")).unwrap(),
        old.bytes
    );
    let c = Connection::open(db_path(&root)).unwrap();
    assert_eq!(
        c.query_row(
            "SELECT id FROM revisions WHERE object_id=?1",
            [&old.document],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        old.revision
    );
    assert_eq!(
        c.query_row(
            "SELECT value_text FROM app_settings WHERE key='workspace_id'",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap(),
        old.workspace
    );
}

#[test]
fn unknown_legacy_structure_cannot_be_authorized_by_recomputed_manifest_hashes() {
    let temp = TempDir::new().unwrap();
    let old = legacy(&temp.path().join("legacy"));
    Connection::open(old.db())
        .unwrap()
        .execute_batch("CREATE TABLE unexpected(value TEXT) STRICT;")
        .unwrap();
    let zip = temp.path().join("altered.zip");
    archive(&old, &zip);
    let root = temp.path().join("active");
    let mut vault = Vault::open(&root).unwrap();
    let active = vault.status().unwrap();
    assert!(vault.inspect_restore(&zip).is_err());
    assert_eq!(vault.status().unwrap().generation_id, active.generation_id);
    let original = fs::read(old.db()).unwrap();
    assert!(Vault::open(&old.root).is_err());
    assert_eq!(fs::read(old.db()).unwrap(), original);
    assert_eq!(
        fs::read_to_string(old.root.join("CURRENT")).unwrap().trim(),
        old.generation
    );
    assert_eq!(
        fs::read_dir(old.root.join("generations")).unwrap().count(),
        1
    );
}

#[test]
fn modified_migrated_restore_candidate_is_rejected_without_activating_it() {
    let temp = TempDir::new().unwrap();
    let old = legacy(&temp.path().join("legacy"));
    let zip = temp.path().join("old.zip");
    archive(&old, &zip);
    let root = temp.path().join("active");
    let mut vault = Vault::open(&root).unwrap();
    let active = vault.status().unwrap();
    let inspection = vault.inspect_restore(&zip).unwrap();
    Connection::open(
        root.join("staging")
            .join(&inspection.token)
            .join("data/patrimoine.sqlite3"),
    )
    .unwrap()
    .execute(
        "UPDATE app_settings SET value_text='Altéré' WHERE key='workspace_name'",
        [],
    )
    .unwrap();
    assert!(vault
        .activate_restore(&inspection.token, true, None)
        .is_err());
    assert_eq!(vault.status().unwrap().generation_id, active.generation_id);
}

#[test]
fn legacy_migration_reads_committed_wal_through_sqlite_backup_api() {
    let temp = TempDir::new().unwrap();
    let old = legacy(&temp.path().join("legacy"));
    let mut writer = Connection::open(old.db()).unwrap();
    writer
        .execute_batch("PRAGMA journal_mode=WAL; PRAGMA wal_autocheckpoint=0;")
        .unwrap();
    let transaction = writer.transaction().unwrap();
    let change = id();
    transaction
        .execute(
            "INSERT INTO change_sets VALUES(?1,2,?2,'Paramètre WAL',?3)",
            params![change, STAMP, id()],
        )
        .unwrap();
    transaction.execute("UPDATE app_settings SET value_text='État validé dans le WAL' WHERE key='workspace_name'",[]).unwrap();
    transaction
        .execute(
            "INSERT INTO audit_events VALUES(?1,?2,?3,NULL,'settings_updated','Paramètre WAL')",
            params![id(), change, old.workspace],
        )
        .unwrap();
    transaction.commit().unwrap();
    assert!(old.db().with_extension("sqlite3-wal").exists());
    let original_main = fs::read(old.db()).unwrap();
    let vault = Vault::open(&old.root).unwrap();
    assert_eq!(
        vault.status().unwrap().settings.workspace_name,
        "État validé dans le WAL"
    );
    assert_eq!(vault.status().unwrap().mutation_sequence, 2);
    assert_eq!(fs::read(old.db()).unwrap(), original_main);
    assert_eq!(
        writer
            .pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        1
    );
    assert_original(&vault, &old, &temp.path().join("wal.pdf"));
}

#[test]
fn interrupted_activation_can_fall_back_to_legacy_then_migrate_a_new_copy() {
    let temp = TempDir::new().unwrap();
    let old = legacy(&temp.path().join("legacy"));
    let original = fs::read(old.db()).unwrap();
    let first = Vault::open(&old.root)
        .unwrap()
        .status()
        .unwrap()
        .generation_id;
    fs::write(
        old.root
            .join("generations")
            .join(&first)
            .join("patrimoine.sqlite3"),
        b"interrupted candidate",
    )
    .unwrap();
    fs::write(
        old.root.join("recovery/activation.json"),
        serde_json::to_vec(&json!({"old_generation":old.generation,"new_generation":first}))
            .unwrap(),
    )
    .unwrap();
    let vault = Vault::open(&old.root).unwrap();
    let state = vault.status().unwrap();
    assert_eq!(state.schema_version, 6);
    assert_ne!(state.generation_id, first);
    assert_ne!(state.generation_id, old.generation);
    assert_eq!(fs::read(old.db()).unwrap(), original);
    assert!(!old.root.join("recovery/activation.json").exists());
    assert_original(&vault, &old, &temp.path().join("recovered.pdf"));
}

#[test]
fn schema_three_open_migrates_a_copy_and_preserves_fx_and_exact_profile_sources() {
    let temp = TempDir::new().unwrap();
    let old = legacy_fx(&temp.path().join("schema-three"));
    let original = fs::read(old.vault.db()).unwrap();
    let vault = Vault::open(&old.vault.root).unwrap();
    assert_eq!(vault.status().unwrap().schema_version, 6);
    assert_ne!(vault.status().unwrap().generation_id, old.vault.generation);
    assert_eq!(fs::read(old.vault.db()).unwrap(), original);
    let state = vault.list_registry().unwrap();
    assert_eq!(state.knowledge_sequence, 9);
    assert_eq!(state.exchange_rates[0].rate, "0.3");
    assert_eq!(
        state.profile_periods[0].profile_revision_id,
        old.initial_profile
    );
    assert_eq!(state.profile_revisions.len(), 2);
    assert!(vault
        .list_registry_at(Some(7))
        .unwrap()
        .exchange_rates
        .is_empty());
    assert_eq!(
        vault.list_registry_at(Some(8)).unwrap().exchange_rates[0].revision_id,
        state.exchange_rates[0].revision_id
    );
    assert!(vault
        .list_properties_at(None)
        .unwrap()
        .properties
        .is_empty());
    assert!(vault.list_rentals_at(None).unwrap().leases.is_empty());
    assert_original(&vault, &old.vault, &temp.path().join("preserved-three.pdf"));
}

#[test]
fn schema_three_archive_migrates_without_modifying_source_or_inventing_property_records() {
    let temp = TempDir::new().unwrap();
    let old = legacy_fx(&temp.path().join("schema-three-source"));
    let zip = temp.path().join("schema-three.zip");
    archive(&old.vault, &zip);
    let bytes = fs::read(&zip).unwrap();
    let mut vault = Vault::open(temp.path().join("current")).unwrap();
    let active = vault.status().unwrap().generation_id;
    let inspected = vault.inspect_restore(&zip).unwrap();
    assert_eq!(inspected.schema_version, 3);
    let restored = vault
        .activate_restore(&inspected.token, true, None)
        .unwrap();
    assert_eq!(restored.schema_version, 6);
    assert_eq!(restored.mutation_sequence, 9);
    assert_ne!(restored.generation_id, active);
    assert_eq!(fs::read(zip).unwrap(), bytes);
    assert_eq!(
        vault.list_registry().unwrap().profile_periods[0].profile_revision_id,
        old.initial_profile
    );
    assert_eq!(vault.list_registry().unwrap().exchange_rates[0].rate, "0.3");
    assert!(vault.list_properties_at(None).unwrap().events.is_empty());
    assert!(vault.list_rentals_at(None).unwrap().dues.is_empty());
}

struct LegacyRentals {
    old: LegacyRegistry,
    property: String,
    unit: String,
    lease: String,
    terms_revision: String,
    due: String,
    payment: String,
}

// Schema 4 is built from the four approved historical scripts, with a real
// unpaid rental and partial payment, not by relabelling a current schema.
fn legacy_rentals(root: &Path) -> LegacyRentals {
    let old = legacy_fx(root);
    let c = Connection::open(old.vault.db()).unwrap();
    c.pragma_update(None, "foreign_keys", false).unwrap();
    c.execute_batch(PROPERTY_RENTALS).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(4,'property_rentals',?1,?2,'0.1.0')",
        params![hash(PROPERTY_RENTALS.as_bytes()), STAMP],
    )
    .unwrap();
    let property = id();
    let rev = legacy_revision(&c, &property, "property", None);
    c.execute("INSERT INTO property_details VALUES(?1,?2,'house','Ville historique','Adresse synthétique','125.5','rented','2020-01-01',NULL,'')",params![rev,old.asset]).unwrap();
    let unit = id();
    let rev = legacy_revision(&c, &unit, "rental_unit", None);
    c.execute("INSERT INTO rental_units VALUES(?1,?2,'LOT-ANCIEN','Unité historique','125.5','2020-01-01',NULL,1,'')",params![rev,old.asset]).unwrap();
    let lease = id();
    let rev = legacy_revision(&c, &lease, "lease", None);
    c.execute(
        "INSERT INTO leases VALUES(?1,?2,?3,'2025-01-01','2026-01-01','Bail ancien synthétique')",
        params![rev, unit, old.person],
    )
    .unwrap();
    let terms = id();
    let terms_revision = legacy_revision(&c, &terms, "lease_terms", None);
    c.execute("INSERT INTO lease_terms VALUES(?1,?2,'1000','0','EUR','monthly',5,'2000','2025-01-01','2026-01-01','')",params![terms_revision,lease]).unwrap();
    let due = id();
    let rev = legacy_revision(&c, &due, "rent_due", None);
    c.execute(
        "INSERT INTO rent_due_keys VALUES(?1,'legacy-rental-january')",
        [&due],
    )
    .unwrap();
    c.execute("INSERT INTO rent_dues VALUES(?1,?2,?3,'2025-01-01','2025-02-01','2025-01-05','1000','0','EUR','')",params![rev,lease,terms_revision]).unwrap();
    let payment = id();
    let rev = legacy_revision(&c, &payment, "rent_payment", None);
    c.execute("INSERT INTO rent_payments VALUES(?1,?2,'receipt','400','EUR','2025-01-05',NULL,?3,'ANCIEN-400','Compte non suivi dans le schéma historique')",params![rev,lease,old.person]).unwrap();
    let allocation = id();
    let rev = legacy_revision(&c, &allocation, "rent_allocation", None);
    c.execute(
        "INSERT INTO rent_allocations VALUES(?1,?2,?3,'rent','400','EUR','2025-01-05')",
        params![rev, payment, due],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 4).unwrap();
    assert!(!c
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    LegacyRentals {
        old,
        property,
        unit,
        lease,
        terms_revision,
        due,
        payment,
    }
}

fn assert_legacy_rentals(vault: &Vault, old: &LegacyRentals) {
    let p = vault.list_properties_at(None).unwrap();
    let r = vault.list_rentals_at(None).unwrap();
    assert_eq!(p.properties[0].id, old.property);
    assert_eq!(p.units[0].id, old.unit);
    assert_eq!(r.leases[0].id, old.lease);
    assert_eq!(r.dues[0].id, old.due);
    assert_eq!(r.dues[0].terms_revision_id, old.terms_revision);
    assert_eq!(r.payments[0].id, old.payment);
    assert_eq!(r.allocations[0].amount, "400");
    assert_eq!(r.knowledge_sequence, 16);
    assert!(vault.list_rentals_at(Some(14)).unwrap().payments.is_empty());
    let t = vault.list_treasury_at(None).unwrap();
    assert!(
        t.accounts.is_empty()
            && t.transactions.is_empty()
            && t.cash_links.is_empty()
            && t.deposits.is_empty()
    );
    assert!(vault.list_debts_at(None).unwrap().contracts.is_empty());
    assert!(vault.list_finance_at(None).unwrap().rent_claims.is_empty());
    assert!(vault.list_finance_at(None).unwrap().coverages.is_empty());
}

#[test]
fn schema_four_open_preserves_unlinked_rentals_without_inventing_bank_or_claim_owners() {
    let temp = TempDir::new().unwrap();
    let old = legacy_rentals(&temp.path().join("schema-four"));
    let original = fs::read(old.old.vault.db()).unwrap();
    let vault = Vault::open(&old.old.vault.root).unwrap();
    assert_eq!(vault.status().unwrap().schema_version, 6);
    assert_ne!(
        vault.status().unwrap().generation_id,
        old.old.vault.generation
    );
    assert_eq!(fs::read(old.old.vault.db()).unwrap(), original);
    assert_legacy_rentals(&vault, &old);
    assert_original(
        &vault,
        &old.old.vault,
        &temp.path().join("original-four.pdf"),
    );
}

#[test]
fn schema_four_archive_migrates_a_copy_retaining_rental_revisions_keys_and_original_bytes() {
    let temp = TempDir::new().unwrap();
    let old = legacy_rentals(&temp.path().join("source-four"));
    let zip = temp.path().join("schema-four.zip");
    archive(&old.old.vault, &zip);
    let original = fs::read(&zip).unwrap();
    let mut vault = Vault::open(temp.path().join("current-five")).unwrap();
    let inspection = vault.inspect_restore(&zip).unwrap();
    assert_eq!(inspection.schema_version, 4);
    let result = vault
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_eq!(result.schema_version, 6);
    assert_eq!(result.mutation_sequence, 16);
    assert_eq!(fs::read(&zip).unwrap(), original);
    assert_legacy_rentals(&vault, &old);
}

struct LegacyFinance {
    old: LegacyRentals,
    account: String,
    opening: String,
    opening_revision: String,
    corrected_opening_revision: String,
    before_correction: u64,
    sequence: u64,
}

// A real schema 5, including versioned bank stock and exact rental/deposit cash
// sources. Its historical SQL remains the authority, not the current migrator.
fn legacy_finance(root: &Path) -> LegacyFinance {
    let old = legacy_rentals(root);
    let c = Connection::open(old.old.vault.db()).unwrap();
    c.pragma_update(None, "foreign_keys", false).unwrap();
    c.execute_batch(FINANCE).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(5,'finance',?1,?2,'0.1.0')",
        params![hash(FINANCE.as_bytes()), STAMP],
    )
    .unwrap();
    let asset = id();
    let revision = legacy_revision(&c, &asset, "asset", None);
    c.execute("INSERT INTO assets VALUES(?1)", [&asset])
        .unwrap();
    c.execute(
        "INSERT INTO asset_profiles VALUES(?1,?2,'Banque historique','FR','bank','Solde sourcé',0)",
        params![revision, asset],
    )
    .unwrap();
    let ownership = id();
    let revision = legacy_revision(&c, &ownership, "ownership", None);
    c.execute(
        "INSERT INTO asset_ownerships VALUES(?1,?2,?3,'1','1','1','1','2025-01-01',NULL)",
        params![revision, asset, old.old.person],
    )
    .unwrap();
    let account = id();
    let revision = legacy_revision(&c, &account, "treasury_account", None);
    c.execute("INSERT INTO treasury_accounts VALUES(?1,?2,'bank','EUR','2025-01-01',?3,'Banque historique','ANCIEN','FR','','',0)",params![revision,asset,old.old.person]).unwrap();
    let opening = id();
    let opening_revision = legacy_revision(&c, &opening, "treasury_transaction", None);
    c.execute("INSERT INTO treasury_transactions VALUES(?1,'opening','2025-01-01','Stock historique','','')",[&opening_revision]).unwrap();
    c.execute(
        "INSERT INTO treasury_entries VALUES(?1,'opening',?2,'100','opening')",
        params![opening_revision, account],
    )
    .unwrap();
    let receipt = id();
    let receipt_revision = legacy_revision(&c, &receipt, "treasury_transaction", None);
    c.execute("INSERT INTO treasury_transactions VALUES(?1,'rent','2025-01-05','Loyer encaissé','ANCIEN-400','')",[&receipt_revision]).unwrap();
    c.execute(
        "INSERT INTO treasury_entries VALUES(?1,'rent',?2,'400','principal')",
        params![receipt_revision, account],
    )
    .unwrap();
    let payment_revision: String = c
        .query_row(
            "SELECT r.id FROM revisions r WHERE r.object_id=?1",
            [&old.payment],
            |r| r.get(0),
        )
        .unwrap();
    let link = id();
    let revision = legacy_revision(&c, &link, "cash_link", None);
    c.execute("INSERT INTO cash_links VALUES(?1,'rent_payment',?2,?3,?4,'rent','400','full','','Source historique exacte')",params![revision,old.payment,payment_revision,receipt_revision]).unwrap();
    let deposit = id();
    let deposit_revision = legacy_revision(&c, &deposit, "deposit_movement", None);
    c.execute("INSERT INTO deposit_movements VALUES(?1,?2,'received','2000','EUR','2025-06-01',?3,NULL,'','Dépôt, hors revenu')",params![deposit_revision,old.lease,old.old.person]).unwrap();
    let deposit_cash = id();
    let deposit_cash_revision = legacy_revision(&c, &deposit_cash, "treasury_transaction", None);
    c.execute(
        "INSERT INTO treasury_transactions VALUES(?1,'receipt','2025-06-01','Dépôt reçu','','')",
        [&deposit_cash_revision],
    )
    .unwrap();
    c.execute(
        "INSERT INTO treasury_entries VALUES(?1,'deposit',?2,'2000','principal')",
        params![deposit_cash_revision, account],
    )
    .unwrap();
    let link = id();
    let revision = legacy_revision(&c, &link, "cash_link", None);
    c.execute("INSERT INTO cash_links VALUES(?1,'deposit_movement',?2,?3,?4,'deposit','2000','full','','Source historique exacte')",params![revision,deposit,deposit_revision,deposit_cash_revision]).unwrap();
    let claim = id();
    let revision = legacy_revision(&c, &claim, "rent_claim", None);
    c.execute(
        "INSERT INTO rent_claims VALUES(?1,?2,'2025-01-01',NULL,NULL,'Droit historique explicite')",
        params![revision, old.due],
    )
    .unwrap();
    c.execute(
        "INSERT INTO rent_claim_parties VALUES(?1,?2,'1','1')",
        params![revision, old.old.person],
    )
    .unwrap();
    let before_correction = c
        .query_row("SELECT max(sequence) FROM change_sets", [], |r| r.get(0))
        .unwrap();
    let corrected_opening_revision = legacy_revision(
        &c,
        &opening,
        "treasury_transaction",
        Some(&opening_revision),
    );
    c.execute("INSERT INTO treasury_transactions VALUES(?1,'opening','2025-01-01','Stock corrigé','','Motif conservé')",[&corrected_opening_revision]).unwrap();
    c.execute(
        "INSERT INTO treasury_entries VALUES(?1,'opening',?2,'120','opening')",
        params![corrected_opening_revision, account],
    )
    .unwrap();
    let sequence = c
        .query_row("SELECT max(sequence) FROM change_sets", [], |r| r.get(0))
        .unwrap();
    c.pragma_update(None, "user_version", 5).unwrap();
    assert!(!c
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
    LegacyFinance {
        old,
        account,
        opening,
        opening_revision,
        corrected_opening_revision,
        before_correction,
        sequence,
    }
}

fn table_values(c: &Connection, table: &str) -> Vec<String> {
    let mut statement = c.prepare(&format!("SELECT * FROM \"{table}\"")).unwrap();
    let columns = statement.column_count();
    let mut values = statement
        .query_map([], |row| {
            let cells = (0..columns)
                .map(|i| match row.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => Ok(json!(null)),
                    rusqlite::types::ValueRef::Integer(v) => Ok(json!(v)),
                    rusqlite::types::ValueRef::Real(v) => Ok(json!(v)),
                    rusqlite::types::ValueRef::Text(v) => Ok(json!(String::from_utf8_lossy(v))),
                    rusqlite::types::ValueRef::Blob(v) => Ok(json!(v)),
                })
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(serde_json::to_string(&cells).unwrap())
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    values.sort();
    values
}

fn assert_legacy_finance(vault: &Vault, old: &LegacyFinance) {
    let original = Connection::open(old.old.old.vault.db()).unwrap();
    let current = Connection::open(db_path(&old.old.old.vault.root)).unwrap();
    let original_tables = tables(&original);
    for table in original_tables
        .keys()
        .filter(|t| t.as_str() != "schema_migrations")
    {
        assert_eq!(
            table_values(&current, table),
            table_values(&original, table),
            "Les données historiques de {table} ont changé"
        );
    }
    for (table, count) in tables(&current) {
        if !original_tables.contains_key(&table) {
            assert_eq!(
                count, 0,
                "Une source du nouveau module a été inventée : {table}"
            );
        }
    }
    let treasury = vault.list_treasury_at(None).unwrap();
    assert_eq!(treasury.knowledge_sequence, old.sequence);
    assert_eq!(treasury.accounts[0].id, old.account);
    assert_eq!(treasury.cash_links.len(), 2);
    assert_eq!(treasury.deposits[0].amount, "2000");
    assert_eq!(
        treasury
            .transactions
            .iter()
            .find(|t| t.id == old.opening)
            .unwrap()
            .revision_id,
        old.corrected_opening_revision
    );
    let earlier = vault.list_treasury_at(Some(old.before_correction)).unwrap();
    assert_eq!(
        earlier
            .transactions
            .iter()
            .find(|t| t.id == old.opening)
            .unwrap()
            .revision_id,
        old.opening_revision
    );
    for (knowledge_sequence, expected) in
        [(None, "2520.00"), (Some(old.before_correction), "2500.00")]
    {
        let report = vault
            .get_treasury_report(patrimoine_domain::TreasuryReportRequest {
                account_id: old.account.clone(),
                date: "2025-12-31".into(),
                knowledge_sequence,
            })
            .unwrap();
        assert_eq!(report.balance_original.as_deref(), Some(expected));
    }
    assert_eq!(
        vault.list_finance_at(None).unwrap().rent_claims[0].due_id,
        old.old.due
    );
    assert_eq!(
        vault.list_rentals_at(None).unwrap().payments[0].id,
        old.old.payment
    );
    assert_eq!(
        vault.list_documents().unwrap()[0].id,
        old.old.old.vault.document
    );
    assert_eq!(
        current
            .query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert!(!current
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .exists([])
        .unwrap());
}

#[test]
fn genuine_schema_five_open_preserves_cash_links_child_rows_and_old_knowledge() {
    let temp = TempDir::new().unwrap();
    let old = legacy_finance(&temp.path().join("schema-five"));
    let original = fs::read(old.old.old.vault.db()).unwrap();
    let vault = Vault::open(&old.old.old.vault.root).unwrap();
    assert_eq!(vault.status().unwrap().schema_version, 6);
    assert_ne!(
        vault.status().unwrap().generation_id,
        old.old.old.vault.generation
    );
    assert_eq!(fs::read(old.old.old.vault.db()).unwrap(), original);
    assert_legacy_finance(&vault, &old);
    assert_original(
        &vault,
        &old.old.old.vault,
        &temp.path().join("original-five.pdf"),
    );
}

#[test]
fn genuine_schema_five_archive_migrates_a_copy_with_bank_revisions_and_exact_sources() {
    let temp = TempDir::new().unwrap();
    let old = legacy_finance(&temp.path().join("schema-five-source"));
    let zip = temp.path().join("schema-five.zip");
    archive(&old.old.old.vault, &zip);
    let original = fs::read(&zip).unwrap();
    let mut vault = Vault::open(temp.path().join("schema-six")).unwrap();
    let inspected = vault.inspect_restore(&zip).unwrap();
    assert_eq!(inspected.schema_version, 5);
    let restored = vault
        .activate_restore(&inspected.token, true, None)
        .unwrap();
    assert_eq!(restored.schema_version, 6);
    assert_eq!(restored.mutation_sequence, old.sequence);
    assert_eq!(fs::read(&zip).unwrap(), original);
    // The restore target differs from the preserved source root. Compare all
    // original rows directly against its activated database before API checks.
    let current = Connection::open(db_path(&temp.path().join("schema-six"))).unwrap();
    let source = Connection::open(old.old.old.vault.db()).unwrap();
    for table in tables(&source)
        .keys()
        .filter(|t| t.as_str() != "schema_migrations")
    {
        assert_eq!(table_values(&current, table), table_values(&source, table));
    }
    let original_tables = tables(&source);
    for (table, count) in tables(&current) {
        if !original_tables.contains_key(&table) {
            assert_eq!(
                count, 0,
                "La restauration a inventé une source du nouveau module : {table}"
            );
        }
    }
    let treasury = vault.list_treasury_at(None).unwrap();
    assert_eq!(treasury.cash_links.len(), 2);
    assert_eq!(
        treasury
            .transactions
            .iter()
            .find(|t| t.id == old.opening)
            .unwrap()
            .revision_id,
        old.corrected_opening_revision
    );
    let old_treasury = vault.list_treasury_at(Some(old.before_correction)).unwrap();
    assert_eq!(
        old_treasury
            .transactions
            .iter()
            .find(|t| t.id == old.opening)
            .unwrap()
            .revision_id,
        old.opening_revision
    );
    assert_original(
        &vault,
        &old.old.old.vault,
        &temp.path().join("restored-five.pdf"),
    );
}
