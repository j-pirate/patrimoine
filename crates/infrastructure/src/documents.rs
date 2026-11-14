//! Versioned document descriptions, original replacements and object associations.
use crate::{archives, db, files::*, finance, registry, Vault};
use patrimoine_application::DocumentService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, TransactionBehavior};
use std::{collections::BTreeMap, fs, path::Path};

fn metadata(description: &str, status: &str) -> Result<()> {
    if description.len() > 10000 || !["active", "archived", "deleted"].contains(&status) {
        return Err(AppError::new("validation", "La description est limitée à 10 000 octets ; le statut documentaire doit être actif, archivé ou supprimé."));
    }
    Ok(())
}
fn record_metadata(record: &DocumentRecord) -> Result<()> {
    metadata(&record.description, &record.status)?;
    if record.original_name.is_empty()
        || record.original_name.len() > 1024
        || record.original_name.contains(['/', '\\', '\0'])
        || [".", ".."].contains(&record.original_name.as_str())
        || record.mime_type.len() > 200
        || record.mime_type.chars().any(char::is_control)
    {
        return Err(invalid(
            "Une métadonnée de version documentaire est invalide.",
        ));
    }
    Ok(())
}
fn interval(from: Option<&str>, to: Option<&str>) -> Result<()> {
    let from = from.ok_or_else(|| {
        AppError::new(
            "validation",
            "Indiquez la date de début de l’association documentaire.",
        )
    })?;
    validate_interval(from, to)?;
    Ok(())
}
fn cutoff(c: &Connection, requested: Option<u64>) -> Result<u64> {
    let latest = db::sequence(c)?;
    let k = requested.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    Ok(k)
}
fn records(c: &Connection, k: u64, history: bool) -> Result<Vec<DocumentRecord>> {
    let cols="v.original_name,v.mime_type,v.description,v.status,b.sha256,b.size_bytes,(SELECT first.recorded_at FROM revisions first JOIN change_sets fc ON fc.id=first.change_set_id WHERE first.object_id=r.object_id ORDER BY fc.sequence LIMIT 1),r.recorded_at";
    let read = |r: &rusqlite::Row<'_>| {
        Ok(DocumentRecord {
            id: r.get(0)?,
            revision_id: r.get(1)?,
            original_name: r.get(2)?,
            mime_type: r.get(3)?,
            description: r.get(4)?,
            status: r.get(5)?,
            sha256: r.get(6)?,
            size_bytes: r.get(7)?,
            added_at: r.get(8)?,
            recorded_at: r.get(9)?,
        })
    };
    let joins = "JOIN document_blobs b ON b.sha256=v.blob_sha256";
    if history {
        registry::profile_history(c, "document_versions", cols, joins, k, read)
    } else {
        registry::load(c, "document_versions", cols, joins, k, read)
    }
}
fn links(c: &Connection, k: u64) -> Result<Vec<DocumentLink>> {
    registry::load(
        c,
        "document_links",
        "v.document_id,v.target_object_id,v.role,v.valid_from,v.valid_to",
        "",
        k,
        |r| {
            Ok(DocumentLink {
                id: r.get(0)?,
                revision_id: r.get(1)?,
                document_id: r.get(2)?,
                target_object_id: r.get(3)?,
                role: r.get(4)?,
                valid_from: r.get(5)?,
                valid_to: r.get(6)?,
            })
        },
    )
}
pub(crate) fn snapshot(c: &Connection, requested: Option<u64>) -> Result<DocumentLibrarySnapshot> {
    let k = cutoff(c, requested)?;
    let mut query=c.prepare("SELECT o.id,o.kind,r.is_void FROM objects o JOIN revisions r ON r.object_id=o.id JOIN change_sets cs ON cs.id=r.change_set_id WHERE o.kind NOT IN ('workspace','document','document_link') AND cs.sequence<=?1 AND NOT EXISTS(SELECT 1 FROM revisions n JOIN change_sets nc ON nc.id=n.change_set_id WHERE n.supersedes_revision_id=r.id AND nc.sequence<=?1) ORDER BY o.kind,o.id").map_err(db::error)?;
    let mut targets = query
        .query_map([k], |r| {
            let id: String = r.get(0)?;
            let kind: String = r.get(1)?;
            Ok(DocumentTarget {
                label: format!("{kind} · {id}"),
                id,
                kind,
                retired: r.get(2)?,
            })
        })
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error)?;
    let version: u32 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(db::error)?;
    if version >= 2 {
        let registry = registry::snapshot(c, Some(k))?;
        let mut labels: BTreeMap<_, _> = registry
            .entities
            .into_iter()
            .map(|e| (e.id, e.display_name))
            .collect();
        labels.extend(registry.assets.into_iter().map(|a| (a.id, a.name)));
        if version >= 4 {
            for (id, asset, reference, name) in registry::load(
                c,
                "rental_units",
                "v.property_asset_id,v.reference,v.name",
                "",
                k,
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                },
            )? {
                let property = labels.get(&asset).map(String::as_str).unwrap_or(&asset);
                labels.insert(id, format!("{property} · {reference} · {name}"));
            }
            for (id, unit, from) in
                registry::load(c, "leases", "v.unit_id,v.valid_from", "", k, |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                    ))
                })?
            {
                let unit = labels.get(&unit).map(String::as_str).unwrap_or(&unit);
                labels.insert(id, format!("Bail · {unit} · {from}"));
            }
        }
        if version >= 5 {
            labels.extend(registry::load(c, "debt_contracts", "v.name", "", k, |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(2)?))
            })?);
            labels.extend(registry::load(
                c,
                "treasury_transactions",
                "v.effective_on,v.label",
                "",
                k,
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        format!("{} · {}", r.get::<_, String>(2)?, r.get::<_, String>(3)?),
                    ))
                },
            )?);
        }
        if version >= 6 {
            labels.extend(registry::load(
                c,
                "metal_lots",
                "v.designation",
                "",
                k,
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(2)?)),
            )?);
        }
        for target in &mut targets {
            if let Some(label) = labels.get(&target.id) {
                target.label = label.clone();
            }
        }
    }
    Ok(DocumentLibrarySnapshot {
        documents: records(c, k, false)?,
        versions: records(c, k, true)?,
        links: links(c, k)?,
        targets,
        knowledge_sequence: k,
    })
}
fn current(c: &Connection, id: &str, expected: &str) -> Result<DocumentRecord> {
    if !valid_id(id) || !valid_id(expected) {
        return Err(AppError::new(
            "validation",
            "Les références du document sont invalides.",
        ));
    }
    let current = records(c, db::sequence(c)?, false)?
        .into_iter()
        .find(|d| d.id == id)
        .ok_or_else(|| AppError::new("not_found", "Ce document n’existe pas dans le coffre."))?;
    if current.revision_id != expected {
        return Err(AppError::new(
            "conflict",
            "Le document a changé. Rechargez sa révision avant de le modifier.",
        ));
    }
    Ok(current)
}
fn insert_version(
    c: &Connection,
    revision: &str,
    record: &DocumentRecord,
    description: &str,
    status: &str,
) -> Result<()> {
    c.execute(
        "INSERT INTO document_versions VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            revision,
            record.sha256,
            record.original_name,
            record.mime_type,
            description,
            status
        ],
    )
    .map_err(db::error)?;
    Ok(())
}
fn version(c: &Connection, document_id: &str, revision_id: &str) -> Result<DocumentRecord> {
    let record = records(c, db::sequence(c)?, true)?
        .into_iter()
        .find(|d| d.id == document_id && d.revision_id == revision_id)
        .ok_or_else(|| {
            AppError::new(
                "not_found",
                "Cette version documentaire n’existe pas dans le coffre.",
            )
        })?;
    record_metadata(&record)?;
    Ok(record)
}
fn source_name(path: &Path) -> Result<String> {
    path.file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty() && v.len() <= 1024 && !v.contains(['/', '\\', '\0']))
        .map(str::to_owned)
        .ok_or_else(|| AppError::new("validation", "Le nom original est invalide."))
}
fn mime(path: &Path) -> String {
    match path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "pdf" => "application/pdf",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "txt" | "md" | "csv" => "text/plain",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "odt" => "application/vnd.oasis.opendocument.text",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "rtf" => "application/rtf",
        _ => "application/octet-stream",
    }
    .into()
}
fn known_document(c: &Connection, document: &str, k: u64) -> Result<bool> {
    Ok(c.query_row("SELECT count(*) FROM document_versions v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.object_id=?1 AND cs.sequence<=?2 AND r.is_void=0 AND v.status<>'deleted' AND NOT EXISTS(SELECT 1 FROM revisions n JOIN change_sets nc ON nc.id=n.change_set_id WHERE n.supersedes_revision_id=r.id AND nc.sequence<=?2)",params![document,k],|r|r.get::<_,u64>(0)).map_err(db::error)?==1)
}
fn eligible_target(c: &Connection, target: &str, k: u64) -> Result<bool> {
    Ok(c.query_row("SELECT count(*) FROM objects o JOIN revisions r ON r.object_id=o.id JOIN change_sets cs ON cs.id=r.change_set_id WHERE o.id=?1 AND o.kind NOT IN ('workspace','document','document_link') AND cs.sequence<=?2 AND r.is_void=0 AND NOT EXISTS(SELECT 1 FROM revisions n JOIN change_sets nc ON nc.id=n.change_set_id WHERE n.supersedes_revision_id=r.id AND nc.sequence<=?2)",params![target,k],|r|r.get::<_,u64>(0)).map_err(db::error)?==1)
}

/// Validate every retained version and the state known when a link was created.
pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    finance::validate_record_history(
        c,
        &[
            ("document", "document_versions"),
            ("document_link", "document_links"),
        ],
    )?;
    let bad_void:u64=c.query_row("SELECT count(*) FROM document_versions v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void<>0",[],|r|r.get(0)).map_err(db::error)?;
    if bad_void != 0 {
        return Err(invalid(
            "Un document est retiré par son statut, sans perdre sa version ni ses originaux.",
        ));
    }
    for record in records(c, db::sequence(c)?, true)? {
        record_metadata(&record)?;
    }
    let mut query=c.prepare("SELECT v.document_id,v.target_object_id,v.role,v.valid_from,v.valid_to,cs.sequence FROM document_links v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.is_void=0 ORDER BY cs.sequence,r.object_id").map_err(db::error)?;
    let rows = query
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Option<String>>(2)?,
                r.get::<_, Option<String>>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, u64>(5)?,
            ))
        })
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error)?;
    for (document, target, role, from, to, k) in rows {
        if role.as_ref().is_some_and(|r| r.len() > 1000) {
            return Err(invalid("Le rôle documentaire est trop long."));
        }
        interval(from.as_deref(), to.as_deref())?;
        if !known_document(c, &document, k)? || !eligible_target(c, &target, k)? {
            return Err(invalid("Une association référence un document retiré, une cible absente ou un objet non encore connu à sa création."));
        }
    }
    let moved:u64=c.query_row("SELECT count(*) FROM (SELECT r.object_id FROM document_links v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.document_id)<>1 OR count(DISTINCT v.target_object_id)<>1)",[],|r|r.get(0)).map_err(db::error)?;
    if moved != 0 {
        return Err(invalid("Une association conserve son document et sa cible d’origine ; retirez-la pour créer un autre lien."));
    }
    Ok(())
}

impl DocumentService for Vault {
    fn list_document_library_at(
        &self,
        knowledge_sequence: Option<u64>,
    ) -> Result<DocumentLibrarySnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_document_metadata(
        &mut self,
        input: SaveDocumentMetadataInput,
    ) -> Result<DocumentRecord> {
        self.ensure_active()?;
        metadata(&input.description, &input.status)?;
        if input.status == "deleted" && !input.confirmed {
            return Err(AppError::new("confirmation_required","Confirmez explicitement le retrait du document. Les originaux et versions seront conservés."));
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let old = current(&tx, &input.id, &input.expected_revision_id)?;
        let (_, revision) = registry::begin(
            &tx,
            Some(&input.id),
            Some(&input.expected_revision_id),
            "document",
            &input.reason,
            false,
        )?;
        insert_version(&tx, &revision, &old, &input.description, &input.status)?;
        validate_history(&tx)?;
        tx.commit().map_err(db::error)?;
        version(&c, &input.id, &revision)
    }
    fn replace_document(
        &mut self,
        input: ReplaceDocumentInput,
        source: &Path,
    ) -> Result<DocumentRecord> {
        self.ensure_active()?;
        metadata(&input.description, "active")?;
        let old = current(
            &db::readonly(&self.db_path())?,
            &input.id,
            &input.expected_revision_id,
        )?;
        if old.status == "deleted" {
            return Err(AppError::new(
                "validation",
                "Rétablissez ce document avant de remplacer son original.",
            ));
        }
        let name = source_name(source)?;
        let before = read_file(source)?.metadata().map_err(io)?;
        if before.len() > archives::MAX_FILE {
            return Err(AppError::new("limit", "Le document dépasse 128 Mio."));
        }
        let stage = self.root.join("staging").join(id());
        directory(&stage)?;
        let temp = stage.join("replacement");
        let result = (|| {
            let (hash, size) = copy_file(source, &temp, archives::MAX_FILE)?;
            let after = read_file(source)?.metadata().map_err(io)?;
            if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
                return Err(AppError::new(
                    "conflict",
                    "Le fichier source a changé pendant sa copie.",
                ));
            }
            let relative = blob_path(&hash);
            let destination = self.generation_path().join(&relative);
            directory(destination.parent().expect("blob parent"))?;
            if destination.exists() {
                let (existing, existing_size) = hash_file(&destination, archives::MAX_FILE)?;
                if existing != hash || existing_size != size {
                    return Err(AppError::new(
                        "integrity",
                        "L’original existant ne correspond pas à son empreinte.",
                    ));
                }
            } else {
                fs::rename(&temp, &destination).map_err(io)?;
                sync_dir(destination.parent().expect("blob parent"))?;
            }
            let mut c = db::connect(&self.db_path())?;
            let tx = c
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(db::error)?;
            let old = current(&tx, &input.id, &input.expected_revision_id)?;
            let (_, revision) = registry::begin(
                &tx,
                Some(&input.id),
                Some(&input.expected_revision_id),
                "document",
                &input.reason,
                false,
            )?;
            let stamp = now();
            tx.execute(
                "INSERT OR IGNORE INTO document_blobs VALUES(?1,?2,?3,?4)",
                params![hash, size, relative, stamp],
            )
            .map_err(db::error)?;
            let next = DocumentRecord {
                sha256: hash,
                original_name: name,
                mime_type: mime(source),
                size_bytes: size,
                ..old
            };
            insert_version(&tx, &revision, &next, &input.description, &next.status)?;
            validate_history(&tx)?;
            tx.commit().map_err(db::error)?;
            version(&c, &input.id, &revision)
        })();
        archives::cleanup(&stage);
        result
    }
    fn save_document_link(&mut self, input: SaveDocumentLinkInput) -> Result<DocumentLink> {
        self.ensure_active()?;
        interval(input.valid_from.as_deref(), input.valid_to.as_deref())?;
        if input.role.as_ref().is_some_and(|r| r.len() > 1000) {
            return Err(AppError::new(
                "validation",
                "Le rôle documentaire est limité à 1 000 octets.",
            ));
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let k = db::sequence(&tx)?;
        if !known_document(&tx, &input.document_id, k)?
            || !eligible_target(&tx, &input.target_object_id, k)?
        {
            return Err(AppError::new(
                "validation",
                "Choisissez un document conservé et une cible métier actuellement connue.",
            ));
        }
        let current_links = links(&tx, k)?;
        if current_links.iter().any(|l| {
            Some(&l.id) != input.id.as_ref()
                && l.document_id == input.document_id
                && l.target_object_id == input.target_object_id
                && l.role == input.role
                && l.valid_from == input.valid_from
                && l.valid_to == input.valid_to
        }) {
            return Err(AppError::new(
                "conflict",
                "Cette association existe déjà ; aucune copie du document n’a été créée.",
            ));
        }
        if let Some(id) = &input.id {
            let old = current_links
                .iter()
                .find(|l| &l.id == id)
                .ok_or_else(|| AppError::new("conflict", "L’association n’est plus courante."))?;
            if old.document_id != input.document_id
                || old.target_object_id != input.target_object_id
            {
                return Err(AppError::new(
                    "validation",
                    "Retirez cette association pour relier un autre document ou une autre cible.",
                ));
            }
        }
        let (id, revision) = registry::begin(
            &tx,
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            "document_link",
            &input.reason,
            false,
        )?;
        tx.execute(
            "INSERT INTO document_links VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                revision,
                input.document_id,
                input.target_object_id,
                input.role,
                input.valid_from,
                input.valid_to
            ],
        )
        .map_err(db::error)?;
        validate_history(&tx)?;
        tx.commit().map_err(db::error)?;
        links(&c, db::sequence(&c)?)?
            .into_iter()
            .find(|l| l.id == id)
            .ok_or_else(|| invalid("L’association enregistrée est introuvable."))
    }
    fn remove_document_link(&mut self, input: VoidRecordInput) -> Result<()> {
        self.ensure_active()?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        if !links(&tx, db::sequence(&tx)?)?
            .iter()
            .any(|l| l.id == input.id)
        {
            return Err(AppError::new(
                "conflict",
                "L’association n’est plus courante.",
            ));
        }
        let (_, revision) = registry::begin(
            &tx,
            Some(&input.id),
            Some(&input.expected_revision_id),
            "document_link",
            &input.reason,
            true,
        )?;
        tx.execute("INSERT INTO document_links SELECT ?1,document_id,target_object_id,role,valid_from,valid_to FROM document_links WHERE revision_id=?2",params![revision,input.expected_revision_id]).map_err(db::error)?;
        validate_history(&tx)?;
        tx.commit().map_err(db::error)?;
        Ok(())
    }
    fn export_document_revision(
        &self,
        document_id: &str,
        revision_id: &str,
        target: &Path,
    ) -> Result<()> {
        self.ensure_active()?;
        let doc = version(&db::readonly(&self.db_path())?, document_id, revision_id)?;
        let parent = destination(target, &self.root)?;
        let temp = parent.join(format!(".patrimoine-{}", id()));
        let result = (|| {
            let (hash, size) = copy_file(
                &self.generation_path().join(blob_path(&doc.sha256)),
                &temp,
                archives::MAX_FILE,
            )?;
            if hash != doc.sha256 || size != doc.size_bytes {
                return Err(AppError::new(
                    "integrity",
                    "L’original ne correspond plus à son empreinte.",
                ));
            }
            publish_file(&temp, target)
        })();
        let _ = fs::remove_file(&temp);
        result
    }
    fn prepare_document_open(
        &self,
        document_id: &str,
        revision_id: &str,
    ) -> Result<PreparedDocument> {
        self.ensure_active()?;
        let doc = version(&db::readonly(&self.db_path())?, document_id, revision_id)?;
        let extension = Path::new(&doc.original_name)
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ![
            "pdf", "jpg", "jpeg", "png", "docx", "odt", "xlsx", "ods", "txt", "md", "csv", "rtf",
        ]
        .contains(&extension.as_str())
        {
            return Err(AppError::new("unsupported_document_open","Ce format reste conservé et exportable. Exportez une copie puis choisissez l’application adaptée pour le consulter."));
        }
        let directory_path = self.root.join("recovery").join("document-views").join(id());
        directory(&directory_path)?;
        let target = directory_path.join(&doc.original_name);
        let result = (|| {
            let (hash, size) = copy_file(
                &self.generation_path().join(blob_path(&doc.sha256)),
                &target,
                archives::MAX_FILE,
            )?;
            if hash != doc.sha256 || size != doc.size_bytes {
                return Err(AppError::new(
                    "integrity",
                    "L’original ne correspond plus à son empreinte.",
                ));
            }
            Ok(PreparedDocument {
                path: target.display().to_string(),
                original_name: doc.original_name,
                mime_type: doc.mime_type,
                sha256: hash,
                size_bytes: size,
            })
        })();
        if result.is_err() {
            archives::cleanup(&directory_path);
        }
        result
    }
}
