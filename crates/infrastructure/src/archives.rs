use crate::{db, files::*};
use patrimoine_domain::{AppError, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashSet},
    fs::{self, File},
    io::{Read, Seek, SeekFrom, Write},
    path::{Component, Path},
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

pub(crate) const MAX_FILE: u64 = 128 * 1024 * 1024;
pub(crate) const MAX_DATABASE: u64 = 256 * 1024 * 1024;
pub(crate) const MAX_TOTAL: u64 = 512 * 1024 * 1024;
pub(crate) const MAX_ENTRIES: usize = 11000;
pub(crate) const MAX_MANIFEST: u64 = 4 * 1024 * 1024;

/// Only the coherent SQLite snapshot has the larger database allowance.
/// Documentary originals and CSV tables retain the existing file limit.
pub(crate) fn entry_limit(name: &str) -> u64 {
    if name == "data/patrimoine.sqlite3" {
        MAX_DATABASE
    } else {
        MAX_FILE
    }
}
pub(crate) fn zip_error(e: zip::result::ZipError) -> AppError {
    invalid(&format!("L’archive ZIP ne peut pas être lue : {e}"))
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub path: String,
    pub size_bytes: u64,
    pub sha256: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub archive_kind: String,
    pub format_version: u32,
    pub schema_version: u32,
    pub app_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation_policy_version: Option<String>,
    pub export_id: String,
    pub created_at: String,
    pub generation_id: String,
    pub mutation_sequence: u64,
    pub document_count: u64,
    pub blob_count: u64,
    pub table_counts: BTreeMap<String, u64>,
    pub files: Vec<Entry>,
}
pub(crate) const README:&str="# Récupération indépendante\n\nExtraire une copie de cette archive ZIP dans un dossier neuf avec un extracteur sûr. Ouvrir data/patrimoine.sqlite3 avec SQLite ou DB Browser for SQLite. Les originaux sont dans documents/sha256/. Joindre document_versions.blob_sha256 à document_blobs.sha256 pour retrouver les noms originaux. Le schéma et les clés sont dans dictionary/schema.json. La base est autonome sans fichier WAL. Les montants et fractions sont des textes décimaux et entiers exacts ; les rapports se recalculent depuis les faits datés et les révisions. Au schéma 6, le registre, le change, les locations, les comptes, les financements, les métaux, les observations de sociétés et leurs liens de trésorerie sont suivis. Les versions documentaires et associations multiples restent conservées, y compris les documents retirés. Les couvertures de périmètre et droits sur créances sont explicites ; absence de source ne signifie pas montant nul. Les empreintes SHA-256 détectent des altérations ; elles ne sont pas une signature d’authenticité.\n";

pub(crate) fn dictionary(c: &Connection, version: u32) -> Result<Vec<u8>> {
    let mut definition = BTreeMap::new();
    let mut tables = c.prepare("SELECT name,sql FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(db::error)?;
    for row in tables
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db::error)?
    {
        let (name, ddl) = row.map_err(db::error)?;
        definition.insert(name, ddl);
    }
    serde_json::to_vec_pretty(&serde_json::json!({"schema_version":version,"tables":definition,"conventions":{"identifiers":"UUID texte stables","dates":"ISO 8601, UTC pour les instants ; intervalles civils [début,fin)","history":"revisions immuables, ordre par change_sets.sequence","document_paths":"documents/sha256/<préfixe>/<sha256>","numbers":"décimaux texte canoniques ; fractions exactes normalisées","financial_modules":if version >= 6 { "registre, change, profils, immobilier, locations, comptes et financements, lots physiques/cotations/coûts proportionnels, observations de sociétés hors bilan et dividendes réellement rapprochés ; couverture explicite avant un net complet" } else if version >= 5 { "registre, change manuel, profils datés, biens, locations, comptes/caisses, écritures atomiques, financements, garanties, dépôts et créances explicites ; couverture de périmètre requise pour un net complet" } else if version >= 4 { "entités, droits, valorisations multidevises, taux manuels, profils datés, biens et locations ; trésorerie et passifs détaillés non suivis" } else if version >= 3 { "entités, droits, valorisations multidevises, taux manuels et profils datés ; passifs détaillés et flux non suivis" } else { "entités, droits et valorisations selon le schéma ; change et passifs détaillés non suivis" },"fx_convention":"1 unité de base = taux unités de cotation ; dernier cours applicable à D et connu à K ; direct, inverse, puis un pivot EUR/USD/CHF/TND/AED ; aucun cours futur ni parité supposée","profile_periods":"[début,fin), référence à une révision descriptive exacte ; aucune datation supposée","calculation_policy_version":"exact-rational-phase8-1","document_history":"versions originales et associations multiples ; suppression logique, aucune purge physique implicite","metal_units":"poids fin = quantité × poids unitaire en grammes × pureté ; kg = 1000 g ; once troy = 31.1034768 g exactement ; coût proportionnel du lot identifié","cash_sources":"lien source_id/source_revision_id et transaction_revision_id/line_id ; une source rapprochée ne crée pas une seconde entrée bancaire","financing":"un contrat, deux faces ; principal distinct des intérêts et frais ; garanties hors passifs certains ; couverture des valeurs globales explicite"}})).map_err(|_|invalid("Le dictionnaire ne peut pas être sérialisé."))
}

fn push_file(writer: &mut ZipWriter<File>, source: &Path, name: &str) -> Result<Entry> {
    let (hash, size) = hash_file(source, entry_limit(name))?;
    writer
        .start_file(
            name,
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o600),
        )
        .map_err(zip_error)?;
    let mut file = read_file(source)?;
    std::io::copy(&mut file, writer).map_err(io)?;
    Ok(Entry {
        path: name.into(),
        size_bytes: size,
        sha256: hash,
    })
}
pub(crate) fn create(
    generation: &Path,
    generation_id: &str,
    output: &Path,
    scratch: &Path,
) -> Result<Manifest> {
    directory(scratch)?;
    let snapshot = scratch.join("snapshot.sqlite3");
    db::copy_snapshot(&generation.join("patrimoine.sqlite3"), &snapshot)?;
    db::validate(&snapshot)?;
    let c = db::readonly(&snapshot)?;
    let counts = db::counts(&c)?;
    let blobs = db::blobs(&c)?;
    let mut manifest = Manifest {
        archive_kind: "full_backup".into(),
        format_version: 1,
        schema_version: db::SCHEMA,
        app_version: db::APP.into(),
        calculation_policy_version: Some("exact-rational-phase8-1".into()),
        export_id: id(),
        created_at: now(),
        generation_id: generation_id.into(),
        mutation_sequence: db::sequence(&c)?,
        document_count: *counts.get("documents").unwrap_or(&0),
        blob_count: blobs.len() as u64,
        table_counts: counts,
        files: Vec::new(),
    };
    let mut writer = ZipWriter::new(new_file(output)?);
    manifest.files.push(push_file(
        &mut writer,
        &snapshot,
        "data/patrimoine.sqlite3",
    )?);
    for (hash, size, relative) in blobs {
        let entry = push_file(&mut writer, &generation.join(&relative), &relative)?;
        if entry.sha256 != hash || entry.size_bytes != size {
            return Err(invalid("Un document est absent ou ne correspond plus à son empreinte. La sauvegarde est abandonnée."));
        }
        manifest.files.push(entry);
    }
    let dictionary = dictionary(&c, db::SCHEMA)?;
    for (name, bytes) in [
        ("recovery/LISEZMOI.md", README.as_bytes()),
        ("dictionary/schema.json", dictionary.as_slice()),
    ] {
        writer
            .start_file(
                name,
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(0o600),
            )
            .map_err(zip_error)?;
        writer.write_all(bytes).map_err(io)?;
        manifest.files.push(Entry {
            path: name.into(),
            size_bytes: bytes.len() as u64,
            sha256: hash_bytes(bytes),
        });
    }
    let bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|_| invalid("Le manifeste ne peut pas être écrit."))?;
    writer
        .start_file(
            "manifest.json",
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .unix_permissions(0o600),
        )
        .map_err(zip_error)?;
    writer.write_all(&bytes).map_err(io)?;
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err(invalid("Le manifeste dépasse la taille autorisée."));
    }
    writer.finish().map_err(zip_error)?.sync_all().map_err(io)?;
    // Une sauvegarde n'est publiée que si notre lecteur peut la restaurer.
    inspect(output, &scratch.join("verification"))?;
    Ok(manifest)
}
pub(crate) fn safe_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() < 512
        && !name.contains(['\\', '\0', ':'])
        && !name.ends_with('/')
        && Path::new(name)
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
        && !name
            .split('/')
            .any(|v| v.is_empty() || v == "." || v == "..")
}
fn allowed(name: &str) -> bool {
    if [
        "manifest.json",
        "data/patrimoine.sqlite3",
        "recovery/LISEZMOI.md",
        "dictionary/schema.json",
    ]
    .contains(&name)
    {
        return true;
    }
    if let Some(hash) = name.rsplit('/').next() {
        return valid_hash(hash) && name == blob_path(hash);
    }
    false
}
pub(crate) fn validate_zip_directory_with(
    path: &Path,
    allowed_name: impl Fn(&str) -> bool,
) -> Result<()> {
    // zip 2.x indexe les entrées par nom et masque les doublons. Contrôler les
    // enregistrements centraux bruts avant de faire confiance à cet index.
    let mut file = read_file(path)?;
    let length = file.metadata().map_err(io)?.len();
    let tail_len = length.min(65557) as usize;
    file.seek(SeekFrom::End(-(tail_len as i64))).map_err(io)?;
    let mut tail = vec![0; tail_len];
    file.read_exact(&mut tail).map_err(io)?;
    let position = (0..tail_len.saturating_sub(21))
        .rev()
        .find(|&p| {
            tail[p..].starts_with(b"PK\x05\x06")
                && p + 22 + u16::from_le_bytes([tail[p + 20], tail[p + 21]]) as usize == tail_len
        })
        .ok_or_else(|| invalid("Le répertoire central ZIP est invalide."))?;
    let end = &tail[position..];
    let u16_at = |p| u16::from_le_bytes([end[p], end[p + 1]]);
    let u32_at = |p| u32::from_le_bytes([end[p], end[p + 1], end[p + 2], end[p + 3]]);
    let count = u16_at(10) as usize;
    let central_size = u32_at(12) as u64;
    let central_start = u32_at(16) as u64;
    let end_offset = length - tail_len as u64 + position as u64;
    if u16_at(4) != 0
        || u16_at(6) != 0
        || u16_at(8) as usize != count
        || count == 0
        || count > MAX_ENTRIES
        || central_size > 16 * 1024 * 1024
        || central_start.checked_add(central_size) != Some(end_offset)
    {
        return Err(invalid("Les archives ZIP64, multi-volumes ou aux en-têtes incohérents ne sont pas prises en charge par le socle."));
    }
    file.seek(SeekFrom::Start(central_start)).map_err(io)?;
    let mut central = vec![0; central_size as usize];
    file.read_exact(&mut central).map_err(io)?;
    let mut offset = 0usize;
    let mut names = HashSet::new();
    let mut records = 0;
    while offset < central.len() {
        let bytes = &central[offset..];
        if bytes.len() < 46 || !bytes.starts_with(b"PK\x01\x02") {
            return Err(invalid("Un en-tête central ZIP est invalide."));
        }
        let field = |p| u16::from_le_bytes([bytes[p], bytes[p + 1]]) as usize;
        let name_len = field(28);
        let record_len = 46 + name_len + field(30) + field(32);
        if record_len > bytes.len() || field(34) != 0 {
            return Err(invalid("Un en-tête ZIP est tronqué ou multi-volumes."));
        }
        let name = std::str::from_utf8(&bytes[46..46 + name_len])
            .map_err(|_| invalid("Les chemins ZIP doivent être en UTF-8."))?;
        if !safe_name(name) || !allowed_name(name) || !names.insert(name.to_owned()) {
            return Err(invalid(
                "Un chemin ZIP est interdit, inattendu ou dupliqué.",
            ));
        }
        offset += record_len;
        records += 1;
    }
    if records != count {
        return Err(invalid("Le nombre brut d’entrées ZIP est incohérent."));
    }
    Ok(())
}
pub(crate) fn inspect(archive: &Path, staging: &Path) -> Result<(Manifest, String)> {
    let (archive_hash, archive_size) = hash_file(archive, MAX_TOTAL)?;
    if archive_size == 0 {
        return Err(invalid("L’archive est vide."));
    }
    validate_zip_directory_with(archive, allowed)?;
    let mut zip = ZipArchive::new(read_file(archive)?).map_err(zip_error)?;
    if zip.len() > MAX_ENTRIES {
        return Err(invalid("L’archive contient trop de fichiers."));
    }
    let mut seen = HashSet::new();
    let mut total = 0u64;
    for index in 0..zip.len() {
        let file = zip.by_index(index).map_err(zip_error)?;
        let name = file.name().map_err(zip_error)?;
        let file_limit = entry_limit(&name);
        if !safe_name(&name) || !allowed(&name) || !seen.insert(name.into_owned()) {
            return Err(invalid(
                "L’archive contient un chemin interdit, un doublon ou un fichier inattendu.",
            ));
        }
        if let Some(mode) = file.unix_mode() {
            let kind = mode & 0o170000;
            if kind != 0 && kind != 0o100000 {
                return Err(invalid(
                    "Les liens et fichiers spéciaux sont interdits dans une sauvegarde.",
                ));
            }
        }
        total = total
            .checked_add(file.size())
            .ok_or_else(|| invalid("Taille d’archive invalide."))?;
        if file.size() > file_limit
            || total > MAX_TOTAL
            || (file.size() > 1024 * 1024
                && file.size() > file.compressed_size().saturating_mul(200))
        {
            return Err(invalid(
                "L’archive dépasse les limites de taille ou de compression.",
            ));
        }
    }
    let mut mf = zip.by_name("manifest.json").map_err(zip_error)?;
    if mf.size() > MAX_MANIFEST {
        return Err(invalid("Le manifeste est trop volumineux."));
    }
    let mut bytes = Vec::new();
    mf.by_ref()
        .take(MAX_MANIFEST + 1)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    drop(mf);
    if bytes.len() as u64 > MAX_MANIFEST {
        return Err(invalid("Le manifeste est trop volumineux."));
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|_| invalid("Le manifeste est absent, invalide ou incompatible."))?;
    if manifest.archive_kind != "full_backup"
        || manifest.format_version != 1
        || !(1..=db::SCHEMA).contains(&manifest.schema_version)
        || !valid_id(&manifest.export_id)
        || !valid_id(&manifest.generation_id)
        || !db::valid_timestamp(&manifest.created_at)
    {
        return Err(invalid(
            "Le type ou la version de sauvegarde n’est pas pris en charge.",
        ));
    }
    let mut declared = HashSet::from(["manifest.json".to_owned()]);
    for entry in &manifest.files {
        if !safe_name(&entry.path)
            || !allowed(&entry.path)
            || !valid_hash(&entry.sha256)
            || !declared.insert(entry.path.clone())
            || entry.size_bytes > entry_limit(&entry.path)
        {
            return Err(invalid("La liste des fichiers du manifeste est invalide."));
        }
    }
    if seen != declared
        || !declared.contains("data/patrimoine.sqlite3")
        || !declared.contains("dictionary/schema.json")
        || !declared.contains("recovery/LISEZMOI.md")
    {
        return Err(invalid(
            "Le contenu de l’archive ne correspond pas au manifeste.",
        ));
    }
    directory(staging)?;
    for entry in &manifest.files {
        let target = staging.join(&entry.path);
        directory(target.parent().expect("parent"))?;
        let mut source = zip.by_name(&entry.path).map_err(zip_error)?;
        if source.size() != entry.size_bytes {
            return Err(invalid(
                "La taille d’un fichier ne correspond pas au manifeste.",
            ));
        }
        let mut output = new_file(&target)?;
        let copied = std::io::copy(&mut source.by_ref().take(entry.size_bytes + 1), &mut output)
            .map_err(io)?;
        output.sync_all().map_err(io)?;
        if copied != entry.size_bytes {
            return Err(invalid("La taille extraite d’un fichier est invalide."));
        }
        let (hash, size) = hash_file(&target, entry_limit(&entry.path))?;
        if hash != entry.sha256 || size != entry.size_bytes {
            return Err(invalid("Une empreinte SHA-256 de l’archive est invalide."));
        }
    }
    validate_staged(staging, &manifest)?;
    // L’archive source n'est plus utilisée : le candidat est une copie autonome validée.
    Ok((manifest, archive_hash))
}
pub(crate) fn validate_staged(staging: &Path, manifest: &Manifest) -> Result<()> {
    let expected: HashSet<_> = manifest
        .files
        .iter()
        .map(|entry| entry.path.clone())
        .collect();
    let mut actual = HashSet::new();
    fn visit(root: &Path, dir: &Path, files: &mut HashSet<String>) -> Result<()> {
        for entry in fs::read_dir(dir).map_err(io)? {
            let entry = entry.map_err(io)?;
            let metadata = fs::symlink_metadata(entry.path()).map_err(io)?;
            if metadata.is_dir() {
                visit(root, &entry.path(), files)?;
            } else if metadata.is_file() {
                files.insert(
                    entry
                        .path()
                        .strip_prefix(root)
                        .map_err(|_| invalid("Chemin de candidat invalide."))?
                        .to_string_lossy()
                        .into_owned(),
                );
            } else {
                return Err(invalid(
                    "Un candidat contient un lien ou un fichier spécial.",
                ));
            }
        }
        Ok(())
    }
    visit(staging, staging, &mut actual)?;
    if actual != expected {
        return Err(invalid(
            "Des fichiers inattendus sont présents dans le candidat de restauration.",
        ));
    }
    for entry in &manifest.files {
        let (hash, size) = hash_file(&staging.join(&entry.path), entry_limit(&entry.path))?;
        if hash != entry.sha256 || size != entry.size_bytes {
            return Err(invalid(
                "Le candidat de restauration a changé. Relancez son analyse.",
            ));
        }
    }
    let dbpath = staging.join("data/patrimoine.sqlite3");
    db::validate(&dbpath)?;
    if db::version(&dbpath)? != manifest.schema_version {
        return Err(invalid(
            "La version de la base diffère de celle du manifeste.",
        ));
    }
    let c = db::readonly(&dbpath)?;
    if db::counts(&c)? != manifest.table_counts || db::sequence(&c)? != manifest.mutation_sequence {
        return Err(invalid(
            "Les nombres d’enregistrements ne correspondent pas au manifeste.",
        ));
    }
    let blobs = db::blobs(&c)?;
    if blobs.len() as u64 != manifest.blob_count
        || manifest.document_count != *manifest.table_counts.get("documents").unwrap_or(&0)
    {
        return Err(invalid("Le nombre de documents est incohérent."));
    }
    let expected: HashSet<_> = blobs.iter().map(|(_, _, path)| path.as_str()).collect();
    let listed: HashSet<_> = manifest
        .files
        .iter()
        .filter(|v| v.path.starts_with("documents/"))
        .map(|v| v.path.as_str())
        .collect();
    if expected != listed {
        return Err(invalid(
            "Les originaux documentaires ne correspondent pas à la base.",
        ));
    }
    for (hash, size, path) in blobs {
        if !manifest
            .files
            .iter()
            .any(|v| v.path == path && v.sha256 == hash && v.size_bytes == size)
        {
            return Err(invalid(
                "Un original documentaire ne correspond pas à la base.",
            ));
        }
    }
    Ok(())
}
pub(crate) fn upgrade_candidate(staging: &Path, source: &Manifest) -> Result<Manifest> {
    // Les empreintes et le schéma d'origine ont déjà été contrôlés. On ne modifie
    // que la copie extraite ; le ZIP d'origine et le coffre actif restent intacts.
    validate_staged(staging, source)?;
    let mut manifest = source.clone();
    if manifest.schema_version == db::SCHEMA {
        return Ok(manifest);
    }
    let path = staging.join("data/patrimoine.sqlite3");
    db::migrate_candidate(&path)?;
    let c = db::readonly(&path)?;
    manifest.schema_version = db::SCHEMA;
    manifest.table_counts = db::counts(&c)?;
    let dictionary = dictionary(&c, db::SCHEMA)?;
    drop(c);
    let dictionary_path = staging.join("dictionary/schema.json");
    let temporary = staging.join("dictionary").join(format!(".schema-{}", id()));
    let mut file = new_file(&temporary)?;
    file.write_all(&dictionary).map_err(io)?;
    file.sync_all().map_err(io)?;
    drop(file);
    fs::rename(&temporary, dictionary_path).map_err(io)?;
    for entry in &mut manifest.files {
        let (hash, size) = hash_file(&staging.join(&entry.path), entry_limit(&entry.path))?;
        entry.sha256 = hash;
        entry.size_bytes = size;
    }
    sync_tree(staging)?;
    validate_staged(staging, &manifest)?;
    Ok(manifest)
}
pub(crate) fn cleanup(path: &Path) {
    let _ = fs::remove_dir_all(path);
}

#[cfg(test)]
#[path = "archives_capacity_tests.rs"]
mod capacity_tests;
