//! Coffre local Linux du socle : SQLite standard, originaux et sauvegardes.
mod archives;
mod companies;
mod csv;
mod dashboard;
mod db;
mod debts;
mod documents;
mod files;
mod finance;
mod metals;
mod phase7;
mod portability;
mod properties;
mod registry;
mod rentals;
mod treasury;
mod xlsx;

use files::*;
use fs2::FileExt;
use patrimoine_application::{validate_settings, VaultService};
use patrimoine_domain::*;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
};

struct Candidate {
    stage: PathBuf,
    manifest: archives::Manifest,
    generation: String,
    sequence: u64,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActivationJournal {
    old_generation: String,
    new_generation: String,
}

pub struct Vault {
    root: PathBuf,
    generation: String,
    _lock: File,
    candidates: HashMap<String, Candidate>,
}
impl Vault {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = root.as_ref().to_path_buf();
        directory(&root)?;
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).map_err(io)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .mode(0o600)
            .custom_flags(libc::O_NOFOLLOW)
            .open(root.join(".lock"))
            .map_err(io)?;
        if !lock.metadata().map_err(io)?.is_file() {
            return Err(AppError::new(
                "validation",
                "Le verrou du coffre doit être un fichier ordinaire.",
            ));
        }
        lock.try_lock_exclusive().map_err(|_| {
            AppError::new(
                "locked",
                "Ce coffre est déjà ouvert dans une autre instance de l’application.",
            )
        })?;
        for child in ["generations", "staging", "recovery"] {
            directory(&root.join(child))?;
        }
        no_symlinks(&root.join("CURRENT"))?;
        let generation = if root.join("CURRENT").exists() {
            let mut selected = current(&root)?;
            let journal_path = root.join("recovery/activation.json");
            if journal_path.exists() {
                let mut json = Vec::new();
                read_file(&journal_path)?
                    .take(1025)
                    .read_to_end(&mut json)
                    .map_err(io)?;
                if json.len() > 1024 {
                    return Err(AppError::new(
                        "recovery",
                        "Le journal de reprise est invalide.",
                    ));
                }
                let journal: ActivationJournal = serde_json::from_slice(&json).map_err(|_| {
                    AppError::new("recovery", "Le journal de reprise est illisible.")
                })?;
                if !valid_id(&journal.old_generation)
                    || !valid_id(&journal.new_generation)
                    || ![
                        journal.old_generation.as_str(),
                        journal.new_generation.as_str(),
                    ]
                    .contains(&selected.as_str())
                {
                    return Err(AppError::new(
                        "recovery",
                        "Le journal de reprise ne correspond pas à CURRENT.",
                    ));
                }
                if db::validate_generation(&root.join("generations").join(&selected)).is_err() {
                    db::validate_generation(
                        &root.join("generations").join(&journal.old_generation),
                    )?;
                    write_current(&root, &journal.old_generation)?;
                    selected = journal.old_generation;
                }
                fs::remove_file(journal_path).map_err(io)?;
                sync_dir(&root.join("recovery"))?;
            }
            selected
        } else {
            if fs::read_dir(root.join("generations"))
                .map_err(io)?
                .next()
                .is_some()
            {
                return Err(AppError::new("recovery","CURRENT est absent alors que des générations existent. Aucune base vide n’a été créée."));
            }
            let selected = id();
            let dir = root.join("generations").join(&selected);
            directory(&dir)?;
            db::create(&dir.join("patrimoine.sqlite3"))?;
            sync_dir(&dir)?;
            sync_dir(&root.join("generations"))?;
            write_current(&root, &selected)?;
            selected
        };
        let mut vault = Self {
            root,
            generation,
            _lock: lock,
            candidates: HashMap::new(),
        };
        db::validate_generation(&vault.generation_path())?;
        vault.upgrade_generation()?;
        vault.status()?;
        Ok(vault)
    }
    fn generation_path(&self) -> PathBuf {
        self.root.join("generations").join(&self.generation)
    }
    fn db_path(&self) -> PathBuf {
        self.generation_path().join("patrimoine.sqlite3")
    }
    fn ensure_active(&self) -> Result<()> {
        if current(&self.root)? != self.generation {
            return Err(AppError::new("recovery","L’état du coffre a changé. Fermez et rouvrez l’application pour terminer la reprise."));
        }
        Ok(())
    }
    fn upgrade_generation(&mut self) -> Result<()> {
        if db::version(&self.db_path())? == db::SCHEMA {
            return Ok(());
        }
        let generation = id();
        let stage = self.root.join("staging").join(id());
        let result = (|| {
            directory(&stage)?;
            db::copy_snapshot(&self.db_path(), &stage.join("patrimoine.sqlite3"))?;
            let source = db::readonly(&self.db_path())?;
            for (hash, size, relative) in db::blobs(&source)? {
                let target = stage.join(&relative);
                directory(target.parent().expect("blob parent"))?;
                let copied = copy_file(
                    &self.generation_path().join(relative),
                    &target,
                    archives::MAX_FILE,
                )?;
                if copied != (hash, size) {
                    return Err(invalid(
                        "Un original a changé pendant la copie préalable à la migration.",
                    ));
                }
            }
            drop(source);
            db::migrate_candidate(&stage.join("patrimoine.sqlite3"))?;
            db::validate_generation(&stage)?;
            sync_tree(&stage)?;
            fs::rename(&stage, self.root.join("generations").join(&generation)).map_err(io)?;
            sync_dir(&self.root.join("generations"))?;
            self.select_generation(generation)
        })();
        archives::cleanup(&stage);
        result
    }
    fn select_generation(&mut self, generation: String) -> Result<()> {
        db::validate_generation(&self.root.join("generations").join(&generation))?;
        let journal = ActivationJournal {
            old_generation: self.generation.clone(),
            new_generation: generation.clone(),
        };
        let journal_path = self.root.join("recovery/activation.json");
        let journal_temp = self
            .root
            .join("recovery")
            .join(format!(".activation-{}", id()));
        let mut file = new_file(&journal_temp)?;
        file.write_all(&serde_json::to_vec(&journal).map_err(|_| {
            AppError::new("recovery", "Le journal de reprise ne peut pas être écrit.")
        })?)
        .map_err(io)?;
        file.sync_all().map_err(io)?;
        publish_file(&journal_temp, &journal_path)?;
        if let Err(error) = write_current(&self.root, &generation) {
            let _ = write_current(&self.root, &self.generation);
            return Err(error);
        }
        let old_generation = std::mem::replace(&mut self.generation, generation);
        if let Err(error) = db::validate_generation(&self.generation_path()) {
            write_current(&self.root, &old_generation)?;
            self.generation = old_generation;
            return Err(error);
        }
        fs::remove_file(journal_path).map_err(io)?;
        sync_dir(&self.root.join("recovery"))?;
        Ok(())
    }
}
impl VaultService for Vault {
    fn status(&self) -> Result<VaultStatus> {
        self.ensure_active()?;
        let c = db::readonly(&self.db_path())?;
        let count = |table: &str| {
            c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| {
                r.get::<_, u64>(0)
            })
            .map_err(db::error)
        };
        Ok(VaultStatus {
            generation_id: self.generation.clone(),
            data_dir: self.root.display().to_string(),
            schema_version: db::SCHEMA,
            app_version: db::APP.into(),
            document_count: count("documents")?,
            mutation_sequence: db::sequence(&c)?,
            settings: db::settings(&c)?,
            country_count: count("countries")?,
            currency_count: count("currencies")?,
        })
    }
    fn save_settings(&mut self, mut settings: Settings) -> Result<Settings> {
        self.ensure_active()?;
        settings.workspace_name = settings.workspace_name.trim().to_owned();
        validate_settings(&settings)?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let (cs, stamp) = db::mutation(&tx, "Modification des paramètres")?;
        for (key, value) in [
            ("workspace_name", settings.workspace_name.as_str()),
            ("display_currency", settings.display_currency.as_str()),
        ] {
            tx.execute(
                "UPDATE app_settings SET value_text=?1,updated_at=?2 WHERE key=?3",
                params![value, stamp, key],
            )
            .map_err(db::error)?;
        }
        let workspace: String = tx
            .query_row(
                "SELECT value_text FROM app_settings WHERE key='workspace_id'",
                [],
                |r| r.get(0),
            )
            .map_err(db::error)?;
        tx.execute("INSERT INTO audit_events VALUES(?1,?2,?3,NULL,'settings_updated','Paramètres de l’espace modifiés')",params![id(),cs,workspace]).map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        Ok(settings)
    }
    fn list_documents(&self) -> Result<Vec<DocumentSummary>> {
        self.ensure_active()?;
        db::documents(&db::readonly(&self.db_path())?)
    }
    fn import_document(&mut self, source: &Path, description: &str) -> Result<DocumentSummary> {
        self.ensure_active()?;
        if description.len() > 10000 {
            return Err(AppError::new(
                "validation",
                "La description est limitée à 10 000 octets.",
            ));
        }
        let name = source
            .file_name()
            .and_then(|v| v.to_str())
            .filter(|v| !v.is_empty() && v.len() <= 1024 && !v.contains(['/', '\\', '\0']))
            .ok_or_else(|| AppError::new("validation", "Le nom du fichier est invalide."))?
            .to_owned();
        let before = read_file(source)?.metadata().map_err(io)?;
        if before.len() > archives::MAX_FILE {
            return Err(AppError::new("limit", "Le document dépasse 128 Mio."));
        }
        let stage = self.root.join("staging").join(id());
        directory(&stage)?;
        let temp = stage.join("import");
        let result = (|| {
            let (hash, size) = copy_file(source, &temp, archives::MAX_FILE)?;
            let after = read_file(source)?.metadata().map_err(io)?;
            if before.len() != after.len() || before.modified().ok() != after.modified().ok() {
                return Err(AppError::new(
                    "conflict",
                    "Le fichier source a changé pendant sa copie. Recommencez l’import.",
                ));
            }
            let relative = blob_path(&hash);
            let target = self.generation_path().join(&relative);
            directory(target.parent().expect("parent"))?;
            if target.exists() {
                let (existing, existing_size) = hash_file(&target, archives::MAX_FILE)?;
                if existing != hash || existing_size != size {
                    return Err(AppError::new(
                        "integrity",
                        "Un original existant ne correspond pas à son empreinte.",
                    ));
                }
            } else {
                fs::rename(&temp, &target).map_err(io)?;
                sync_dir(target.parent().expect("parent"))?;
            }
            let mime = match source
                .extension()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase()
                .as_str()
            {
                "pdf" => "application/pdf",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "txt" | "md" => "text/plain",
                _ => "application/octet-stream",
            }
            .to_owned();
            let document = id();
            let revision = id();
            let mut c = db::connect(&self.db_path())?;
            let tx = c
                .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
                .map_err(db::error)?;
            let (cs, stamp) = db::mutation(&tx, "Import d’un document")?;
            tx.execute(
                "INSERT OR IGNORE INTO document_blobs VALUES(?1,?2,?3,?4)",
                params![hash, size, relative, stamp],
            )
            .map_err(db::error)?;
            tx.execute(
                "INSERT INTO objects VALUES(?1,'document',?2)",
                params![document, stamp],
            )
            .map_err(db::error)?;
            tx.execute("INSERT INTO documents VALUES(?1)", [&document])
                .map_err(db::error)?;
            tx.execute(
                "INSERT INTO revisions VALUES(?1,?2,?3,?4,NULL,0,NULL)",
                params![revision, document, cs, stamp],
            )
            .map_err(db::error)?;
            tx.execute(
                "INSERT INTO document_versions VALUES(?1,?2,?3,?4,?5,'active')",
                params![revision, hash, name, mime, description],
            )
            .map_err(db::error)?;
            tx.execute("INSERT INTO audit_events VALUES(?1,?2,?3,?4,'document_imported','Document importé')",params![id(),cs,document,revision]).map_err(db::error)?;
            tx.commit().map_err(db::error)?;
            Ok(DocumentSummary {
                id: document,
                original_name: name,
                mime_type: mime,
                size_bytes: size,
                sha256: hash,
                added_at: stamp,
            })
        })();
        archives::cleanup(&stage);
        result
    }
    fn export_document(&self, id: &str, target: &Path) -> Result<()> {
        let parent = destination(target, &self.root)?;
        let doc = self
            .list_documents()?
            .into_iter()
            .find(|v| v.id == id)
            .ok_or_else(|| {
                AppError::new(
                    "not_found",
                    "Ce document n’existe pas dans le coffre actif.",
                )
            })?;
        let temp = parent.join(format!(".patrimoine-{}", files::id()));
        let result = (|| {
            let (hash, size) = copy_file(
                &self.generation_path().join(blob_path(&doc.sha256)),
                &temp,
                archives::MAX_FILE,
            )?;
            if hash != doc.sha256 || size != doc.size_bytes {
                return Err(AppError::new(
                    "integrity",
                    "L’original documentaire ne correspond plus à son empreinte.",
                ));
            }
            publish_file(&temp, target)
        })();
        let _ = fs::remove_file(&temp);
        result
    }
    fn create_backup(&mut self, target: &Path) -> Result<BackupSummary> {
        self.ensure_active()?;
        if target.extension().and_then(|v| v.to_str()) != Some("zip") {
            return Err(AppError::new(
                "validation",
                "La sauvegarde doit avoir l’extension .zip.",
            ));
        }
        let parent = destination(target, &self.root)?;
        let temp = parent.join(format!(".patrimoine-{}.zip", id()));
        let scratch = self.root.join("staging").join(id());
        let result = (|| {
            let manifest =
                archives::create(&self.generation_path(), &self.generation, &temp, &scratch)?;
            let (hash, size) = hash_file(&temp, 512 * 1024 * 1024)?;
            publish_file(&temp, target)?;
            Ok(BackupSummary {
                path: target.display().to_string(),
                sha256: hash,
                size_bytes: size,
                document_count: manifest.document_count,
                blob_count: manifest.blob_count,
                created_at: manifest.created_at,
            })
        })();
        let _ = fs::remove_file(&temp);
        archives::cleanup(&scratch);
        result
    }
    fn inspect_restore(&mut self, archive: &Path) -> Result<RestoreInspection> {
        // Limiter les candidats retenus : une nouvelle analyse remplace l'analyse précédente.
        for (_, candidate) in self.candidates.drain() {
            archives::cleanup(&candidate.stage);
        }
        let token = id();
        let stage = self.root.join("staging").join(&token);
        let result = (|| {
            let (source_manifest, archive_hash) = archives::inspect(archive, &stage)?;
            let manifest = archives::upgrade_candidate(&stage, &source_manifest)?;
            let state = self.status()?;
            let settings = db::settings(&db::readonly(&stage.join("data/patrimoine.sqlite3"))?)?;
            let result = RestoreInspection {
                token: token.clone(),
                archive_sha256: archive_hash,
                created_at: source_manifest.created_at.clone(),
                schema_version: source_manifest.schema_version,
                app_version: source_manifest.app_version.clone(),
                document_count: manifest.document_count,
                blob_count: manifest.blob_count,
                settings,
                source_generation_id: source_manifest.generation_id.clone(),
                current_generation_id: state.generation_id.clone(),
                mutation_sequence: state.mutation_sequence,
            };
            self.candidates.insert(
                token.clone(),
                Candidate {
                    stage: stage.clone(),
                    manifest,
                    generation: state.generation_id,
                    sequence: state.mutation_sequence,
                },
            );
            Ok(result)
        })();
        if result.is_err() {
            archives::cleanup(&stage);
        }
        result
    }
    fn activate_restore(
        &mut self,
        token: &str,
        confirmed: bool,
        safety_backup: Option<&Path>,
    ) -> Result<VaultStatus> {
        if !confirmed {
            return Err(AppError::new(
                "confirmation_required",
                "Confirmez explicitement le remplacement intégral du coffre.",
            ));
        }
        let candidate = self.candidates.get(token).ok_or_else(|| {
            AppError::new(
                "conflict",
                "Cette analyse n’est plus disponible. Analysez de nouveau la sauvegarde.",
            )
        })?;
        let state = self.status()?;
        if candidate.generation != state.generation_id
            || candidate.sequence != state.mutation_sequence
        {
            return Err(AppError::new("conflict","Le coffre a changé depuis l’analyse. Analysez de nouveau la sauvegarde avant de confirmer."));
        }
        archives::validate_staged(&candidate.stage, &candidate.manifest)?;
        if let Some(path) = safety_backup {
            self.create_backup(path)?;
        }
        let candidate = self.candidates.remove(token).expect("candidate validated");
        let new_generation = id();
        let new_path = self.root.join("generations").join(&new_generation);
        directory(&new_path)?;
        // Aucune connexion durable ne reste ouverte ; la génération arrêtée est conservée.
        let old = db::connect(&self.db_path())?;
        old.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(db::error)?;
        drop(old);
        fs::rename(
            candidate.stage.join("data/patrimoine.sqlite3"),
            new_path.join("patrimoine.sqlite3"),
        )
        .map_err(io)?;
        if candidate.stage.join("documents").exists() {
            fs::rename(
                candidate.stage.join("documents"),
                new_path.join("documents"),
            )
            .map_err(io)?;
        }
        // Les fichiers extraits ont été fsync ; synchroniser les répertoires après renommage.
        sync_tree(&new_path)?;
        sync_dir(&self.root.join("generations"))?;
        let result = self
            .select_generation(new_generation)
            .and_then(|()| self.status());
        archives::cleanup(&candidate.stage);
        result
    }
}
impl Drop for Vault {
    fn drop(&mut self) {
        for (_, candidate) in self.candidates.drain() {
            archives::cleanup(&candidate.stage);
        }
    }
}
