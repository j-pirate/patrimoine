//! Coherent portable exports and CSV reconstruction into an isolated candidate.
use crate::{archives, csv, db, files::*, Candidate, Vault};
use patrimoine_application::{PortabilityService, VaultService};
use patrimoine_domain::{ExportSummary, RestoreInspection, Result};
use rusqlite::{types::Value, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub(crate) const POLICY: &str = "exact-rational-phase8-1";

pub(crate) struct ExportSnapshot {
    pub path: PathBuf,
    pub generation_path: PathBuf,
    pub knowledge_sequence: u64,
    pub table_counts: BTreeMap<String, u64>,
    pub blobs: Vec<(String, u64, String)>,
}

pub(crate) fn capture_export_snapshot(vault: &Vault, scratch: &Path) -> Result<ExportSnapshot> {
    vault.ensure_active()?;
    directory(scratch)?;
    let path = scratch.join("snapshot.sqlite3");
    db::copy_snapshot(&vault.db_path(), &path)?;
    db::validate(&path)?;
    let c = db::readonly(&path)?;
    Ok(ExportSnapshot {
        path,
        generation_path: vault.generation_path(),
        knowledge_sequence: db::sequence(&c)?,
        table_counts: db::counts(&c)?,
        blobs: db::blobs(&c)?,
    })
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Dialect {
    encoding: String,
    delimiter: String,
    quote: String,
    line_endings: String,
    null_markers: String,
}
fn dialect() -> Dialect {
    Dialect {
        encoding: "UTF-8 sans BOM".into(),
        delimiter: ",".into(),
        quote: "\"".into(),
        line_endings: "CRLF".into(),
        null_markers: "<colonne>__is_null : 0=valeur, 1=NULL ; NULL exige une cellule vide".into(),
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CsvManifest {
    archive_kind: String,
    format_version: u32,
    schema_version: u32,
    app_version: String,
    calculation_policy_version: String,
    export_id: String,
    created_at: String,
    generation_id: String,
    mutation_sequence: u64,
    document_count: u64,
    blob_count: u64,
    table_counts: BTreeMap<String, u64>,
    tables: Vec<csv::Table>,
    dialect: Dialect,
    files: Vec<archives::Entry>,
}

const CSV_README:&str="# Archive CSV intégrale — format 1\n\nCette archive contient toutes les tables, leurs versions historiques et les originaux documentaires, indépendamment des filtres de l’interface. UTF-8 sans BOM, séparateur virgule, guillemets doubles, fins de ligne CRLF. Les colonnes `<nom>__is_null` distinguent NULL (1 avec cellule métier vide) d’une chaîne vide réelle (0). Les décimaux et identifiants sont du texte exact. Les cellules commençant par =, +, - ou @ ne sont pas modifiées : utiliser le XLSX de consultation pour un tableur, ou importer les CSV avec types texte explicites.\n\nLire dictionary/schema.json pour les types, clés, unités et relations. Pour reconstruire le coffre, choisir Analyser une archive CSV dans l’application, vérifier le résumé puis confirmer le remplacement. La génération courante et l’archive source ne sont jamais modifiées pendant l’analyse. Le format 1 prend en charge le schéma 6, premier schéma exporté en CSV. Aucun SQL de cette archive n’est exécuté. Chaque table est limitée à 128 Mio et deux millions de cellules ; chaque cellule à 1 Mio ; total décompressé maximal 512 Mio. Aucun dépassement n’est tronqué silencieusement.\n\nLes documents se trouvent dans documents/sha256/. Joindre document_versions.blob_sha256 à document_blobs.sha256 pour retrouver leurs noms et vérifier SHA-256. Les versions retirées restent conservées. Les SHA-256 détectent une altération mais ne sont pas une signature d’authenticité.\n";

fn dictionary(c: &Connection, tables: &[csv::Table]) -> Result<Vec<u8>> {
    let mut value: serde_json::Value =
        serde_json::from_slice(&archives::dictionary(c, db::SCHEMA)?)
            .map_err(|_| invalid("Le dictionnaire du schéma est invalide."))?;
    value["csv_format_version"] = 1.into();
    value["csv_tables"] = serde_json::to_value(tables)
        .map_err(|_| invalid("Le dictionnaire CSV ne peut pas être écrit."))?;
    value["csv_dialect"] = serde_json::to_value(dialect())
        .map_err(|_| invalid("Le dialecte CSV ne peut pas être écrit."))?;
    let mut relations = BTreeMap::<String, Vec<serde_json::Value>>::new();
    for table in tables {
        let rows=c.prepare(&format!("PRAGMA foreign_key_list(\"{}\")",table.name)).map_err(db::error)?.query_map([],|r|Ok(serde_json::json!({"id":r.get::<_,u32>(0)?,"sequence":r.get::<_,u32>(1)?,"target_table":r.get::<_,String>(2)?,"from":r.get::<_,String>(3)?,"to":r.get::<_,Option<String>>(4)?}))).map_err(db::error)?.collect::<std::result::Result<Vec<_>,_>>().map_err(db::error)?;
        relations.insert(table.name.clone(), rows);
    }
    value["relations"] = serde_json::to_value(relations)
        .map_err(|_| invalid("Le dictionnaire des relations est invalide."))?;
    serde_json::to_vec_pretty(&value)
        .map_err(|_| invalid("Le dictionnaire CSV ne peut pas être écrit."))
}

fn write_bytes(path: &Path, bytes: &[u8]) -> Result<()> {
    directory(path.parent().expect("parent"))?;
    let mut f = new_file(path)?;
    f.write_all(bytes).map_err(io)?;
    f.sync_all().map_err(io)
}
fn zip_entry(writer: &mut ZipWriter<File>, source: &Path, name: &str) -> Result<archives::Entry> {
    writer
        .start_file(
            name,
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o600),
        )
        .map_err(archives::zip_error)?;
    let mut source = read_file(source)?;
    let mut digest = Sha256::new();
    let mut size_bytes = 0u64;
    let mut bytes = [0u8; 65536];
    loop {
        let n = source.read(&mut bytes).map_err(io)?;
        if n == 0 {
            break;
        }
        size_bytes += n as u64;
        if size_bytes > archives::MAX_FILE {
            return Err(invalid("Un fichier CSV ou documentaire dépasse 128 Mio."));
        }
        writer.write_all(&bytes[..n]).map_err(io)?;
        digest.update(&bytes[..n]);
    }
    Ok(archives::Entry {
        path: name.into(),
        size_bytes,
        sha256: format!("{:x}", digest.finalize()),
    })
}

fn allowed(name: &str, tables: &[csv::Table]) -> bool {
    if [
        "manifest.json",
        "dictionary/schema.json",
        "recovery/LISEZMOI.md",
    ]
    .contains(&name)
    {
        return true;
    }
    if tables
        .iter()
        .any(|t| name == format!("tables/{}.csv", t.name))
    {
        return true;
    }
    name.rsplit('/')
        .next()
        .is_some_and(|hash| valid_hash(hash) && name == blob_path(hash))
}

fn export_csv(vault: &Vault, target: &Path) -> Result<ExportSummary> {
    if target.extension().and_then(|v| v.to_str()) != Some("zip") {
        return Err(invalid(
            "L’archive CSV intégrale doit porter l’extension .zip.",
        ));
    }
    let parent = destination(target, &vault.root)?;
    let temp = parent.join(format!(".patrimoine-csv-{}.zip", id()));
    let scratch = vault.root.join("staging").join(id());
    let result = (|| {
        let snapshot = capture_export_snapshot(vault, &scratch)?;
        let c = db::readonly(&snapshot.path)?;
        let tables = csv::tables(&c)?;
        let mut manifest = CsvManifest {
            archive_kind: "structured_csv".into(),
            format_version: 1,
            schema_version: db::SCHEMA,
            app_version: db::APP.into(),
            calculation_policy_version: POLICY.into(),
            export_id: id(),
            created_at: now(),
            generation_id: vault.generation.clone(),
            mutation_sequence: snapshot.knowledge_sequence,
            document_count: *snapshot.table_counts.get("documents").unwrap_or(&0),
            blob_count: snapshot.blobs.len() as u64,
            table_counts: snapshot.table_counts.clone(),
            tables: tables.clone(),
            dialect: dialect(),
            files: Vec::new(),
        };
        directory(&scratch.join("tables"))?;
        let mut writer = ZipWriter::new(new_file(&temp)?);
        let mut total = 0u64;
        for table in &tables {
            let relative = format!("tables/{}.csv", table.name);
            let path = scratch.join(&relative);
            csv::write_table(&c, table, &path)?;
            let entry = zip_entry(&mut writer, &path, &relative)?;
            total += entry.size_bytes;
            manifest.files.push(entry);
            if total > archives::MAX_TOTAL {
                return Err(invalid("L’archive CSV dépasse 512 Mio décompressés."));
            }
        }
        for (hash, size, relative) in &snapshot.blobs {
            let entry = zip_entry(
                &mut writer,
                &snapshot.generation_path.join(relative),
                relative,
            )?;
            if &entry.sha256 != hash || &entry.size_bytes != size {
                return Err(invalid("Un original documentaire manque ou son empreinte diffère ; export CSV abandonné."));
            }
            total += entry.size_bytes;
            manifest.files.push(entry);
            if total > archives::MAX_TOTAL {
                return Err(invalid("L’archive CSV dépasse 512 Mio décompressés."));
            }
        }
        for (relative, bytes) in [
            ("dictionary/schema.json", dictionary(&c, &tables)?),
            ("recovery/LISEZMOI.md", CSV_README.as_bytes().to_vec()),
        ] {
            let path = scratch.join(relative);
            write_bytes(&path, &bytes)?;
            let entry = zip_entry(&mut writer, &path, relative)?;
            total += entry.size_bytes;
            manifest.files.push(entry);
        }
        let bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|_| invalid("Le manifeste CSV ne peut pas être écrit."))?;
        if bytes.len() as u64 > archives::MAX_MANIFEST
            || total + bytes.len() as u64 > archives::MAX_TOTAL
            || manifest.files.len() + 1 > archives::MAX_ENTRIES
        {
            return Err(invalid(
                "Les limites de taille de l’archive CSV sont dépassées.",
            ));
        }
        writer
            .start_file(
                "manifest.json",
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Stored)
                    .unix_permissions(0o600),
            )
            .map_err(archives::zip_error)?;
        writer.write_all(&bytes).map_err(io)?;
        writer
            .finish()
            .map_err(archives::zip_error)?
            .sync_all()
            .map_err(io)?;
        let (sha256, size_bytes) = hash_file(&temp, archives::MAX_TOTAL)?;
        // Verify the directory and every emitted hash before publication.
        archives::validate_zip_directory_with(&temp, |n| allowed(n, &tables))?;
        publish_file(&temp, target)?;
        Ok(ExportSummary {
            path: target.display().to_string(),
            sha256,
            size_bytes,
            format: "structured_csv".into(),
            schema_version: db::SCHEMA,
            knowledge_sequence: snapshot.knowledge_sequence,
            table_count: tables.len() as u64,
            row_count: snapshot.table_counts.values().sum(),
            document_count: manifest.document_count,
            blob_count: manifest.blob_count,
        })
    })();
    let _ = fs::remove_file(&temp);
    archives::cleanup(&scratch);
    result
}

fn extract_csv(archive: &Path, raw: &Path, tables: &[csv::Table]) -> Result<(CsvManifest, String)> {
    let (archive_hash, size) = hash_file(archive, archives::MAX_TOTAL)?;
    if size == 0 {
        return Err(invalid("L’archive CSV est vide."));
    }
    archives::validate_zip_directory_with(archive, |n| allowed(n, tables))?;
    let mut zip = ZipArchive::new(read_file(archive)?).map_err(archives::zip_error)?;
    let mut actual = HashSet::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let f = zip.by_index(i).map_err(archives::zip_error)?;
        let name = f.name().map_err(archives::zip_error)?;
        if !archives::safe_name(&name)
            || !allowed(&name, tables)
            || !actual.insert(name.into_owned())
        {
            return Err(invalid(
                "Un chemin CSV est interdit, inattendu ou dupliqué.",
            ));
        }
        if f.unix_mode()
            .is_some_and(|m| !matches!(m & 0o170000, 0 | 0o100000))
        {
            return Err(invalid("Les liens et fichiers spéciaux sont interdits."));
        }
        total = total
            .checked_add(f.size())
            .ok_or_else(|| invalid("Taille d’archive CSV invalide."))?;
        if f.size() > archives::MAX_FILE
            || total > archives::MAX_TOTAL
            || (f.size() > 1024 * 1024 && f.size() > f.compressed_size().saturating_mul(200))
        {
            return Err(invalid(
                "Les limites de taille ou compression CSV sont dépassées.",
            ));
        }
    }
    let mut entry = zip.by_name("manifest.json").map_err(archives::zip_error)?;
    if entry.size() > archives::MAX_MANIFEST {
        return Err(invalid("Le manifeste CSV est trop volumineux."));
    }
    let mut bytes = Vec::new();
    entry
        .by_ref()
        .take(archives::MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    drop(entry);
    let manifest: CsvManifest = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("Le manifeste CSV est invalide ou incompatible."))?;
    if manifest.archive_kind != "structured_csv"
        || manifest.format_version != 1
        || manifest.schema_version != db::SCHEMA
        || manifest.calculation_policy_version != POLICY
        || !valid_id(&manifest.export_id)
        || !valid_id(&manifest.generation_id)
        || manifest.tables != tables
        || manifest.dialect != dialect()
    {
        return Err(invalid(
            "Le format CSV, le schéma ou le dictionnaire des colonnes n’est pas pris en charge.",
        ));
    }
    let mut declared = HashSet::from(["manifest.json".to_owned()]);
    for entry in &manifest.files {
        if !allowed(&entry.path, tables)
            || !archives::safe_name(&entry.path)
            || !valid_hash(&entry.sha256)
            || !declared.insert(entry.path.clone())
            || entry.size_bytes > archives::MAX_FILE
        {
            return Err(invalid("La liste des fichiers CSV est invalide."));
        }
    }
    if actual != declared
        || !declared.contains("dictionary/schema.json")
        || !declared.contains("recovery/LISEZMOI.md")
        || tables
            .iter()
            .any(|t| !declared.contains(&format!("tables/{}.csv", t.name)))
    {
        return Err(invalid(
            "Le contenu CSV ne correspond pas à son manifeste ou une table manque.",
        ));
    }
    directory(raw)?;
    for entry in &manifest.files {
        let path = raw.join(&entry.path);
        directory(path.parent().expect("parent"))?;
        let mut source = zip.by_name(&entry.path).map_err(archives::zip_error)?;
        if source.size() != entry.size_bytes {
            return Err(invalid(
                "La taille déclarée d’un fichier CSV diffère du ZIP.",
            ));
        }
        let mut target = new_file(&path)?;
        let copied = std::io::copy(&mut source.by_ref().take(entry.size_bytes + 1), &mut target)
            .map_err(io)?;
        target.sync_all().map_err(io)?;
        let (hash, size) = hash_file(&path, archives::MAX_FILE)?;
        if copied != entry.size_bytes || size != entry.size_bytes || hash != entry.sha256 {
            return Err(invalid(
                "Une empreinte ou taille de fichier CSV est invalide.",
            ));
        }
    }
    Ok((manifest, archive_hash))
}

fn import_order<'a>(c: &Connection, tables: &'a [csv::Table]) -> Result<Vec<&'a csv::Table>> {
    // Use only the approved local schema. Loading children first leaves deferred
    // references that SQLite resolves by scanning unindexed child columns for
    // every later parent insertion (notably audit_events -> revisions).
    let names: HashSet<_> = tables.iter().map(|table| table.name.as_str()).collect();
    let mut dependencies = BTreeMap::new();
    for table in tables {
        let mut query = c
            .prepare(&format!(
                "PRAGMA foreign_key_list(\"{}\")",
                table.name.replace('"', "\"\"")
            ))
            .map_err(db::error)?;
        let parents = query
            .query_map([], |row| row.get::<_, String>(2))
            .map_err(db::error)?
            .collect::<std::result::Result<HashSet<_>, _>>()
            .map_err(db::error)?;
        if parents
            .iter()
            .any(|parent| !names.contains(parent.as_str()))
        {
            return Err(invalid("Une dépendance du schéma CSV local est absente."));
        }
        dependencies.insert(
            table.name.as_str(),
            parents
                .into_iter()
                .filter(|parent| parent != &table.name)
                .collect::<HashSet<_>>(),
        );
    }
    let mut ordered = Vec::with_capacity(tables.len());
    let mut imported = HashSet::new();
    while ordered.len() < tables.len() {
        let before = ordered.len();
        for table in tables {
            if !imported.contains(table.name.as_str())
                && dependencies[table.name.as_str()]
                    .iter()
                    .all(|parent| imported.contains(parent.as_str()))
            {
                imported.insert(table.name.as_str());
                ordered.push(table);
            }
        }
        if ordered.len() == before {
            return Err(invalid(
                "Le schéma CSV local contient un cycle de dépendances entre tables.",
            ));
        }
    }
    Ok(ordered)
}

fn import_tables(
    c: &mut Connection,
    raw: &Path,
    tables: &[csv::Table],
    manifest: &CsvManifest,
) -> Result<()> {
    let ordered = import_order(c, tables)?;
    let tx = c.transaction().map_err(db::error)?;
    tx.execute_batch("PRAGMA defer_foreign_keys=ON;")
        .map_err(db::error)?;
    for table in tables {
        tx.execute(&format!("DELETE FROM \"{}\"", table.name), [])
            .map_err(db::error)?;
    }
    for table in ordered {
        let mut bytes = Vec::new();
        read_file(&raw.join(format!("tables/{}.csv", table.name)))?
            .take(archives::MAX_FILE + 1)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        let rows =
            csv::parse(&bytes).map_err(|e| csv::context(&table.name, 1, "CSV", &e.message))?;
        if rows.first() != Some(&table.headers()) {
            return Err(csv::context(
                &table.name,
                1,
                "en-tête",
                "Colonnes manquantes, inconnues ou dans un ordre différent du format 1.",
            ));
        }
        if manifest.table_counts.get(&table.name) != Some(&((rows.len() - 1) as u64)) {
            return Err(csv::context(
                &table.name,
                1,
                "nombre de lignes",
                "Le nombre de lignes diffère du manifeste.",
            ));
        }
        let sql = format!(
            "INSERT INTO \"{}\" ({}) VALUES({})",
            table.name,
            table
                .columns
                .iter()
                .map(|c| format!("\"{}\"", c.name))
                .collect::<Vec<_>>()
                .join(","),
            vec!["?"; table.columns.len()].join(",")
        );
        let mut insert = tx.prepare(&sql).map_err(db::error)?;
        for (row_index, row) in rows.iter().enumerate().skip(1) {
            if row.len() != table.headers().len() {
                return Err(csv::context(
                    &table.name,
                    row_index + 1,
                    "CSV",
                    "Le nombre de cellules diffère de l’en-tête.",
                ));
            }
            let mut nullable = table.columns.len();
            let mut values = Vec::new();
            for (index, column) in table.columns.iter().enumerate() {
                let null = if column.nullable {
                    let flag = &row[nullable];
                    nullable += 1;
                    match flag.as_str() {
                        "0" => false,
                        "1" => true,
                        _ => {
                            return Err(csv::context(
                                &table.name,
                                row_index + 1,
                                &format!("{}__is_null", column.name),
                                "Le marqueur doit valoir 0 ou 1.",
                            ))
                        }
                    }
                } else {
                    false
                };
                if null {
                    if !row[index].is_empty() {
                        return Err(csv::context(
                            &table.name,
                            row_index + 1,
                            &column.name,
                            "Une valeur NULL doit porter une cellule métier vide.",
                        ));
                    }
                    values.push(Value::Null);
                } else if column.declared_type == "INTEGER" {
                    let parsed = row[index].parse::<i64>().map_err(|_| {
                        csv::context(
                            &table.name,
                            row_index + 1,
                            &column.name,
                            "Un entier décimal canonique signé sur 64 bits est nécessaire.",
                        )
                    })?;
                    if parsed.to_string() != row[index] {
                        return Err(csv::context(
                            &table.name,
                            row_index + 1,
                            &column.name,
                            "L’entier n’est pas canonique.",
                        ));
                    }
                    values.push(Value::Integer(parsed));
                } else {
                    values.push(Value::Text(row[index].clone()));
                }
            }
            insert
                .execute(rusqlite::params_from_iter(values))
                .map_err(|e| {
                    csv::context(
                        &table.name,
                        row_index + 1,
                        "contraintes SQL",
                        &e.to_string(),
                    )
                })?;
        }
    }
    let fk: bool = tx
        .prepare("PRAGMA foreign_key_check")
        .map_err(db::error)?
        .query([])
        .map_err(db::error)?
        .next()
        .map_err(db::error)?
        .is_some();
    if fk {
        return Err(invalid(
            "Une référence entre tables CSV est absente ; aucune ligne n’a été importée.",
        ));
    }
    tx.commit().map_err(db::error)
}

fn inspect_csv(vault: &mut Vault, archive: &Path) -> Result<RestoreInspection> {
    for (_, candidate) in vault.candidates.drain() {
        archives::cleanup(&candidate.stage);
    }
    let token = id();
    let stage = vault.root.join("staging").join(&token);
    let result = (|| {
        directory(&stage.join("data"))?;
        let path = stage.join("data/patrimoine.sqlite3");
        db::create(&path)?;
        let mut c = db::connect(&path)?;
        let tables = csv::tables(&c)?;
        let raw = stage.join("csv-input");
        let (source, archive_hash) = extract_csv(archive, &raw, &tables)?;
        let expected_dictionary = dictionary(&c, &tables)?;
        let found = fs::read(raw.join("dictionary/schema.json")).map_err(io)?;
        let expected: serde_json::Value = serde_json::from_slice(&expected_dictionary)
            .map_err(|_| invalid("Dictionnaire local CSV invalide."))?;
        let found: serde_json::Value =
            serde_json::from_slice(&found).map_err(|_| invalid("Dictionnaire CSV illisible."))?;
        if expected != found {
            return Err(invalid("Le dictionnaire CSV ne correspond pas au schéma approuvé ; aucun SQL d’archive n’est exécuté."));
        }
        import_tables(&mut c, &raw, &tables, &source)?;
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(db::error)?;
        drop(c);
        db::validate(&path)?;
        let c = db::readonly(&path)?;
        if db::counts(&c)? != source.table_counts || db::sequence(&c)? != source.mutation_sequence {
            return Err(invalid(
                "Les nombres de lignes ou la connaissance importée diffèrent du manifeste.",
            ));
        }
        let blobs = db::blobs(&c)?;
        let listed = source
            .files
            .iter()
            .filter(|e| e.path.starts_with("documents/"))
            .map(|e| e.path.clone())
            .collect::<HashSet<_>>();
        if blobs.len() as u64 != source.blob_count
            || *source.table_counts.get("documents").unwrap_or(&0) != source.document_count
            || listed != blobs.iter().map(|(_, _, p)| p.clone()).collect()
        {
            return Err(invalid(
                "Les originaux documentaires CSV ne correspondent pas à leurs métadonnées.",
            ));
        }
        for (hash, size, relative) in &blobs {
            if !source
                .files
                .iter()
                .any(|e| &e.path == relative && &e.sha256 == hash && &e.size_bytes == size)
            {
                return Err(invalid(
                    "Un original documentaire CSV manque ou ne correspond pas à sa référence.",
                ));
            }
        }
        // Only after the exact source has passed all checks, append an explicit
        // technical provenance event in the candidate, never in the live vault.
        drop(c);
        let mut imported = db::connect(&path)?;
        let tx = imported.transaction().map_err(db::error)?;
        let workspace: String = tx
            .query_row(
                "SELECT value_text FROM app_settings WHERE key='workspace_id'",
                [],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        let (change, _stamp) = db::mutation(&tx, "Réimportation CSV intégrale")?;
        tx.execute(
            "INSERT INTO audit_events VALUES(?1,?2,?3,NULL,'csv_imported',?4)",
            rusqlite::params![
                id(),
                change,
                workspace,
                format!(
                    "Export {} ; SHA-256 {} ; connaissances source {}",
                    source.export_id, archive_hash, source.mutation_sequence
                )
            ],
        )
        .map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        imported
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE;")
            .map_err(db::error)?;
        drop(imported);
        let c = db::readonly(&path)?;
        if raw.join("documents").exists() {
            fs::rename(raw.join("documents"), stage.join("documents")).map_err(io)?;
        }
        write_bytes(
            &stage.join("dictionary/schema.json"),
            &archives::dictionary(&c, db::SCHEMA)?,
        )?;
        write_bytes(
            &stage.join("recovery/LISEZMOI.md"),
            archives::README.as_bytes(),
        )?;
        let mut files = Vec::new();
        for relative in std::iter::once("data/patrimoine.sqlite3".to_owned())
            .chain(blobs.iter().map(|(_, _, p)| p.clone()))
            .chain([
                "dictionary/schema.json".into(),
                "recovery/LISEZMOI.md".into(),
            ])
        {
            let (sha256, size_bytes) =
                hash_file(&stage.join(&relative), archives::entry_limit(&relative))?;
            files.push(archives::Entry {
                path: relative,
                sha256,
                size_bytes,
            });
        }
        let manifest = archives::Manifest {
            archive_kind: "full_backup".into(),
            format_version: 1,
            schema_version: db::SCHEMA,
            app_version: source.app_version,
            calculation_policy_version: Some(source.calculation_policy_version),
            export_id: source.export_id,
            created_at: source.created_at,
            generation_id: source.generation_id,
            mutation_sequence: db::sequence(&c)?,
            document_count: source.document_count,
            blob_count: source.blob_count,
            table_counts: db::counts(&c)?,
            files,
        };
        archives::cleanup(&raw);
        sync_tree(&stage)?;
        archives::validate_staged(&stage, &manifest)?;
        let state = vault.status()?;
        let settings = db::settings(&c)?;
        let inspection = RestoreInspection {
            token: token.clone(),
            archive_sha256: archive_hash,
            created_at: manifest.created_at.clone(),
            schema_version: manifest.schema_version,
            app_version: manifest.app_version.clone(),
            document_count: manifest.document_count,
            blob_count: manifest.blob_count,
            settings,
            source_generation_id: manifest.generation_id.clone(),
            current_generation_id: state.generation_id.clone(),
            mutation_sequence: state.mutation_sequence,
        };
        drop(c);
        vault.candidates.insert(
            token.clone(),
            Candidate {
                stage: stage.clone(),
                manifest,
                generation: state.generation_id,
                sequence: state.mutation_sequence,
            },
        );
        Ok(inspection)
    })();
    if result.is_err() {
        archives::cleanup(&stage);
    }
    result
}

impl PortabilityService for Vault {
    fn export_csv_archive(&mut self, target: &Path) -> Result<ExportSummary> {
        export_csv(self, target)
    }
    fn export_xlsx(&mut self, target: &Path) -> Result<ExportSummary> {
        crate::xlsx::export_xlsx(self, target)
    }
    fn inspect_csv_import(&mut self, archive: &Path) -> Result<RestoreInspection> {
        inspect_csv(self, archive)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_import_orders_all_62_local_tables_before_their_children() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("schema.sqlite3");
        db::create(&path).unwrap();
        let c = db::connect(&path).unwrap();
        let tables = csv::tables(&c).unwrap();
        let ordered = import_order(&c, &tables).unwrap();
        assert_eq!(tables.len(), 62);
        assert_eq!(ordered.len(), 62);
        let positions: BTreeMap<_, _> = ordered
            .iter()
            .enumerate()
            .map(|(index, table)| (table.name.as_str(), index))
            .collect();
        assert_eq!(positions.len(), 62);
        for table in &tables {
            let mut statement = c
                .prepare(&format!("PRAGMA foreign_key_list(\"{}\")", table.name))
                .unwrap();
            let parents = statement
                .query_map([], |row| row.get::<_, String>(2))
                .unwrap();
            for parent in parents {
                let parent = parent.unwrap();
                if parent != table.name {
                    assert!(positions[parent.as_str()] < positions[table.name.as_str()]);
                }
            }
        }
        assert!(positions["revisions"] < positions["audit_events"]);
        assert!(positions["documents"] < positions["document_links"]);
    }

    #[test]
    fn csv_import_rejects_intertable_cycles_but_allows_self_references() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE a(id TEXT PRIMARY KEY, parent TEXT REFERENCES a(id));")
            .unwrap();
        assert_eq!(
            import_order(&c, &csv::tables(&c).unwrap()).unwrap().len(),
            1
        );
        c.execute_batch("CREATE TABLE b(id TEXT PRIMARY KEY, parent TEXT REFERENCES c(id)); CREATE TABLE c(id TEXT PRIMARY KEY, parent TEXT REFERENCES b(id));")
            .unwrap();
        let error = import_order(&c, &csv::tables(&c).unwrap()).unwrap_err();
        assert!(error.message.contains("cycle"));
    }
}
