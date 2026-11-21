use patrimoine_domain::{AppError, ErrorCode, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::ffi::OsStrExt,
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

pub(crate) fn io(e: std::io::Error) -> AppError {
    AppError::new(
        ErrorCode::Io,
        format!("L’opération sur le fichier a échoué : {e}"),
    )
}
pub(crate) fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::InvalidArchive, message)
}
pub(crate) fn id() -> String {
    Uuid::new_v4().to_string()
}
pub(crate) fn valid_id(s: &str) -> bool {
    Uuid::parse_str(s).is_ok_and(|v| v.to_string() == s)
}
pub(crate) fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("format RFC3339")
}
pub(crate) fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub(crate) fn valid_hash(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|v| v.is_ascii_digit() || (b'a'..=b'f').contains(&v))
}
pub(crate) fn blob_path(hash: &str) -> String {
    format!("documents/sha256/{}/{hash}", &hash[..2])
}

pub(crate) fn no_symlinks(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
    {
        return Err(AppError::new(
            ErrorCode::Validation,
            "Un chemin local absolu sans traversée est nécessaire.",
        ));
    }
    let mut prefix = PathBuf::new();
    for c in path.components() {
        prefix.push(c);
        match fs::symlink_metadata(&prefix) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(AppError::new(
                    ErrorCode::Validation,
                    "Les liens symboliques ne sont pas acceptés pour cette opération.",
                ))
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(io(e)),
        }
    }
    Ok(())
}
pub(crate) fn directory(path: &Path) -> Result<()> {
    no_symlinks(path)?;
    if path.exists() {
        if !path.is_dir() {
            return Err(AppError::new(
                ErrorCode::Io,
                "Un dossier attendu est remplacé par un fichier.",
            ));
        }
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| io(std::io::Error::other("Dossier sans parent")))?;
    directory(parent)?;
    fs::DirBuilder::new().mode(0o700).create(path).map_err(io)?;
    // Publier chaque composant avant que son enfant puisse être référencé.
    sync_dir(parent)
}
pub(crate) fn sync_blob_directory(generation: &Path, parent: &Path) -> Result<()> {
    if !parent.starts_with(generation) {
        return Err(invalid(
            "Le dossier documentaire est hors de sa génération.",
        ));
    }
    directory(parent)?;
    // Même un blob réutilisé peut provenir d'une opération interrompue. Refaire
    // la synchronisation de toute sa chaîne avant la référence transactionnelle.
    let mut next = Some(parent);
    while let Some(path) = next {
        sync_dir(path)?;
        if path == generation {
            return Ok(());
        }
        next = path.parent();
    }
    Err(invalid("La chaîne documentaire est incomplète."))
}
pub(crate) fn read_file(path: &Path) -> Result<File> {
    no_symlinks(path)?;
    let f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .map_err(io)?;
    if !f.metadata().map_err(io)?.is_file() {
        return Err(AppError::new(
            ErrorCode::Validation,
            "Choisissez un fichier ordinaire.",
        ));
    }
    Ok(f)
}
pub(crate) fn new_file(path: &Path) -> Result<File> {
    no_symlinks(path)?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(io)
}
pub(crate) fn sync_dir(path: &Path) -> Result<()> {
    #[cfg(test)]
    test_faults::check("sync", path).map_err(io)?;
    File::open(path).map_err(io)?.sync_all().map_err(io)
}
pub(crate) fn sync_tree(path: &Path) -> Result<()> {
    for entry in fs::read_dir(path).map_err(io)? {
        let entry = entry.map_err(io)?;
        let meta = fs::symlink_metadata(entry.path()).map_err(io)?;
        if meta.is_dir() {
            sync_tree(&entry.path())?;
        } else if meta.is_file() {
            read_file(&entry.path())?.sync_all().map_err(io)?;
        } else {
            return Err(invalid(
                "Un fichier spécial interdit est présent dans le coffre.",
            ));
        }
    }
    sync_dir(path)
}
pub(crate) fn hash_file(path: &Path, limit: u64) -> Result<(String, u64)> {
    let mut f = read_file(path)?;
    let mut digest = Sha256::new();
    let mut size = 0u64;
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        size = size
            .checked_add(n as u64)
            .ok_or_else(|| invalid("Fichier trop volumineux."))?;
        if size > limit {
            return Err(invalid(
                "La taille du fichier dépasse la limite de sécurité.",
            ));
        }
        digest.update(&buf[..n]);
    }
    Ok((format!("{:x}", digest.finalize()), size))
}
pub(crate) fn copy_file(source: &Path, destination: &Path, limit: u64) -> Result<(String, u64)> {
    let mut input = read_file(source)?;
    let mut output = new_file(destination)?;
    let mut digest = Sha256::new();
    let mut size = 0u64;
    let mut buf = [0u8; 65536];
    loop {
        let n = input.read(&mut buf).map_err(io)?;
        if n == 0 {
            break;
        }
        size += n as u64;
        if size > limit {
            return Err(AppError::new(
                ErrorCode::Limit,
                "Le fichier dépasse la limite autorisée.",
            ));
        }
        output.write_all(&buf[..n]).map_err(io)?;
        digest.update(&buf[..n]);
    }
    output.sync_all().map_err(io)?;
    Ok((format!("{:x}", digest.finalize()), size))
}
pub(crate) fn destination(path: &Path, root: &Path) -> Result<PathBuf> {
    no_symlinks(path)?;
    let parent = path.parent().filter(|p| p.is_dir()).ok_or_else(|| {
        AppError::new(
            ErrorCode::Validation,
            "Le dossier de destination doit exister.",
        )
    })?;
    if path.file_name().is_none() || path.exists() {
        return Err(AppError::new(
            ErrorCode::Validation,
            "Choisissez un nouveau fichier ; aucun fichier existant n’est écrasé.",
        ));
    }
    if path.starts_with(root) {
        return Err(AppError::new(
            ErrorCode::Validation,
            "La destination doit être située en dehors du coffre de l’application.",
        ));
    }
    Ok(parent.to_path_buf())
}
pub(crate) fn publish_file(temp: &Path, target: &Path) -> Result<()> {
    // Linux garantit l'exclusion avec RENAME_NOREPLACE, y compris sur les
    // supports qui ne proposent pas de liens physiques (notamment FAT/exFAT).
    // L'ancien noyau/FS peut seulement utiliser le lien atomique équivalent.
    let moved = match rename_exclusive(temp, target) {
        Ok(()) => true,
        Err(e)
            if [
                Some(libc::ENOSYS),
                Some(libc::EINVAL),
                Some(libc::EOPNOTSUPP),
            ]
            .contains(&e.raw_os_error()) =>
        {
            fs::hard_link(temp, target).map_err(|e| AppError::new(
                    ErrorCode::PublicationUnavailable,
                    format!("La publication exclusive n’est pas disponible sur ce support : {e}. Choisissez un disque local, puis copiez le fichier vérifié sur ce support."),
                ))?;
            false
        }
        Err(e) => return Err(io(e)),
    };
    if let Err(error) = sync_dir(target.parent().expect("parent")) {
        return Err(AppError::new(ErrorCode::PublicationUncertain, format!(
            "Le fichier {} a été publié, mais sa conservation après une panne n’a pas pu être confirmée : {} Vérifiez ce fichier avant de réessayer et choisissez un nouveau nom ; aucun fichier existant ne sera écrasé.",
            target.display(), error.message,
        )));
    }
    if !moved {
        let _ = fs::remove_file(temp);
    }
    Ok(())
}
fn rename_exclusive(source: &Path, target: &Path) -> std::io::Result<()> {
    let source = std::ffi::CString::new(source.as_os_str().as_bytes())?;
    let target = std::ffi::CString::new(target.as_os_str().as_bytes())?;
    let result = unsafe {
        libc::renameat2(
            libc::AT_FDCWD,
            source.as_ptr(),
            libc::AT_FDCWD,
            target.as_ptr(),
            libc::RENAME_NOREPLACE,
        )
    };
    if result == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}
pub(crate) fn remove_activation_journal(path: &Path) -> Result<()> {
    #[cfg(test)]
    test_faults::check("remove", path).map_err(io)?;
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(io(e)),
    }
}
pub(crate) fn write_current(root: &Path, generation: &str) -> Result<()> {
    if !valid_id(generation) {
        return Err(invalid("Identifiant de génération invalide."));
    }
    let tmp = root.join(format!("CURRENT.{}", id()));
    let result = (|| {
        let mut f = new_file(&tmp)?;
        f.write_all(format!("{generation}\n").as_bytes())
            .map_err(io)?;
        f.sync_all().map_err(io)?;
        #[cfg(test)]
        test_faults::check("current_rename", root).map_err(io)?;
        fs::rename(&tmp, root.join("CURRENT")).map_err(io)?;
        sync_dir(root)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

#[cfg(test)]
pub(crate) mod test_faults {
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
    };
    thread_local! { static FAILURE: RefCell<Option<(&'static str, PathBuf)>> = const { RefCell::new(None) }; }
    thread_local! { static SYNCS: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) }; }
    pub(crate) fn take_syncs() -> Vec<PathBuf> {
        SYNCS.with(|syncs| std::mem::take(&mut *syncs.borrow_mut()))
    }
    pub(crate) fn fail(operation: &'static str, path: &Path) {
        FAILURE.with(|failure| *failure.borrow_mut() = Some((operation, path.to_owned())));
    }
    pub(super) fn check(operation: &str, path: &Path) -> std::io::Result<()> {
        if operation == "sync" {
            SYNCS.with(|syncs| syncs.borrow_mut().push(path.to_owned()));
        }
        FAILURE.with(|failure| {
            let mut failure = failure.borrow_mut();
            if failure
                .as_ref()
                .is_some_and(|(op, p)| *op == operation && p == path)
            {
                failure.take();
                Err(std::io::Error::from_raw_os_error(libc::EIO))
            } else {
                Ok(())
            }
        })
    }
}

#[cfg(test)]
#[path = "files_durability_tests.rs"]
mod durability_tests;
pub(crate) fn current(root: &Path) -> Result<String> {
    let mut value = String::new();
    read_file(&root.join("CURRENT"))?
        .take(39)
        .read_to_string(&mut value)
        .map_err(io)?;
    if value.len() > 37 {
        return Err(AppError::new(
            ErrorCode::Recovery,
            "Le pointeur CURRENT est trop long.",
        ));
    }
    let value = value.strip_suffix('\n').unwrap_or(&value);
    if !valid_id(value) {
        return Err(AppError::new(
            ErrorCode::Recovery,
            "Le pointeur CURRENT est invalide. Le coffre n’a pas été remplacé par une base vide.",
        ));
    }
    Ok(value.to_owned())
}
