use patrimoine_domain::{AppError, Result};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, OpenOptionsExt},
    path::{Component, Path, PathBuf},
};
use uuid::Uuid;

pub(crate) fn io(e: std::io::Error) -> AppError {
    AppError::new("io", format!("L’opération sur le fichier a échoué : {e}"))
}
pub(crate) fn invalid(message: &str) -> AppError {
    AppError::new("invalid_archive", message)
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
            "validation",
            "Un chemin local absolu sans traversée est nécessaire.",
        ));
    }
    let mut prefix = PathBuf::new();
    for c in path.components() {
        prefix.push(c);
        match fs::symlink_metadata(&prefix) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(AppError::new(
                    "validation",
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
                "io",
                "Un dossier attendu est remplacé par un fichier.",
            ));
        }
        return Ok(());
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
        .map_err(io)
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
            "validation",
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
                "limit",
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
    let parent = path
        .parent()
        .filter(|p| p.is_dir())
        .ok_or_else(|| AppError::new("validation", "Le dossier de destination doit exister."))?;
    if path.file_name().is_none() || path.exists() {
        return Err(AppError::new(
            "validation",
            "Choisissez un nouveau fichier ; aucun fichier existant n’est écrasé.",
        ));
    }
    if path.starts_with(root) {
        return Err(AppError::new(
            "validation",
            "La destination doit être située en dehors du coffre de l’application.",
        ));
    }
    Ok(parent.to_path_buf())
}
pub(crate) fn publish_file(temp: &Path, target: &Path) -> Result<()> {
    // Le lien atomique échoue si la destination apparaît entre temps : aucun écrasement.
    fs::hard_link(temp, target).map_err(io)?;
    sync_dir(target.parent().expect("parent"))?;
    fs::remove_file(temp).map_err(io)?;
    Ok(())
}
pub(crate) fn write_current(root: &Path, generation: &str) -> Result<()> {
    if !valid_id(generation) {
        return Err(invalid("Identifiant de génération invalide."));
    }
    let tmp = root.join(format!("CURRENT.{}", id()));
    let mut f = new_file(&tmp)?;
    f.write_all(format!("{generation}\n").as_bytes())
        .map_err(io)?;
    f.sync_all().map_err(io)?;
    fs::rename(tmp, root.join("CURRENT")).map_err(io)?;
    sync_dir(root)
}
pub(crate) fn current(root: &Path) -> Result<String> {
    let mut value = String::new();
    read_file(&root.join("CURRENT"))?
        .take(39)
        .read_to_string(&mut value)
        .map_err(io)?;
    if value.len() > 37 {
        return Err(AppError::new(
            "recovery",
            "Le pointeur CURRENT est trop long.",
        ));
    }
    let value = value.strip_suffix('\n').unwrap_or(&value);
    if !valid_id(value) {
        return Err(AppError::new(
            "recovery",
            "Le pointeur CURRENT est invalide. Le coffre n’a pas été remplacé par une base vide.",
        ));
    }
    Ok(value.to_owned())
}
