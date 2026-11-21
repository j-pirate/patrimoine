//! Contrats du coffre local et des modules patrimoniaux livrés.
use serde::{Deserialize, Serialize};
use std::fmt;

mod companies;
mod consolidation;
mod dashboard;
mod debts;
mod documents;
pub mod exact;
mod finance;
mod financial_adapters;
mod fx;
mod guided;
mod metals;
mod operations;
mod portability;
mod profiles;
mod property;
mod registry;
mod rentals;
mod treasury;
mod validation;
mod variation;
pub use companies::*;
pub use consolidation::*;
pub use dashboard::*;
pub use debts::*;
pub use documents::*;
pub use finance::*;
pub use financial_adapters::*;
pub use fx::*;
pub use guided::*;
pub use metals::*;
pub use operations::*;
pub use portability::*;
pub use profiles::*;
pub use property::*;
pub use registry::*;
pub use rentals::*;
pub use treasury::*;
pub use validation::validate_registry;
pub use variation::*;

/// Stable known error codes. AppError keeps unknown/legacy codes verbatim.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ErrorCode {
    #[serde(rename = "conflict")]
    Conflict,
    #[serde(rename = "validation")]
    Validation,
    #[serde(rename = "io")]
    Io,
    #[serde(rename = "database")]
    Database,
    #[serde(rename = "schema_version")]
    SchemaVersion,
    #[serde(rename = "integrity")]
    Integrity,
    #[serde(rename = "not_found")]
    NotFound,
    #[serde(rename = "invalid_archive")]
    InvalidArchive,
    #[serde(rename = "invalid_csv")]
    InvalidCsv,
    #[serde(rename = "publication_unavailable")]
    PublicationUnavailable,
    #[serde(rename = "publication_uncertain")]
    PublicationUncertain,
    #[serde(rename = "recovery_required")]
    RecoveryRequired,
    #[serde(rename = "recovery")]
    Recovery,
    #[serde(rename = "locked")]
    Locked,
    #[serde(rename = "lock_io")]
    LockIo,
    #[serde(rename = "capacity")]
    Capacity,
    #[serde(rename = "limit")]
    Limit,
    #[serde(rename = "confirmation_required")]
    ConfirmationRequired,
    #[serde(rename = "snapshot_timeout")]
    SnapshotTimeout,
    #[serde(rename = "document_open")]
    DocumentOpen,
    #[serde(rename = "unsupported_document_open")]
    UnsupportedDocumentOpen,
    #[serde(rename = "xlsx_export")]
    XlsxExport,
    #[serde(rename = "FILE_SCOPE")]
    FileScope,
    #[serde(rename = "VAULT_LOCK")]
    VaultLock,
    #[serde(rename = "TASK_FAILED")]
    TaskFailed,
}
impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Conflict => "conflict",
            Self::Validation => "validation",
            Self::Io => "io",
            Self::Database => "database",
            Self::SchemaVersion => "schema_version",
            Self::Integrity => "integrity",
            Self::NotFound => "not_found",
            Self::InvalidArchive => "invalid_archive",
            Self::InvalidCsv => "invalid_csv",
            Self::PublicationUnavailable => "publication_unavailable",
            Self::PublicationUncertain => "publication_uncertain",
            Self::RecoveryRequired => "recovery_required",
            Self::Recovery => "recovery",
            Self::Locked => "locked",
            Self::LockIo => "lock_io",
            Self::Capacity => "capacity",
            Self::Limit => "limit",
            Self::ConfirmationRequired => "confirmation_required",
            Self::SnapshotTimeout => "snapshot_timeout",
            Self::DocumentOpen => "document_open",
            Self::UnsupportedDocumentOpen => "unsupported_document_open",
            Self::XlsxExport => "xlsx_export",
            Self::FileScope => "FILE_SCOPE",
            Self::VaultLock => "VAULT_LOCK",
            Self::TaskFailed => "TASK_FAILED",
        }
    }
    pub fn parse(code: &str) -> Option<Self> {
        match code {
            "conflict" => Some(Self::Conflict),
            "validation" => Some(Self::Validation),
            "io" => Some(Self::Io),
            "database" => Some(Self::Database),
            "schema_version" => Some(Self::SchemaVersion),
            "integrity" => Some(Self::Integrity),
            "not_found" => Some(Self::NotFound),
            "invalid_archive" => Some(Self::InvalidArchive),
            "invalid_csv" => Some(Self::InvalidCsv),
            "publication_unavailable" => Some(Self::PublicationUnavailable),
            "publication_uncertain" => Some(Self::PublicationUncertain),
            "recovery_required" => Some(Self::RecoveryRequired),
            "recovery" => Some(Self::Recovery),
            "locked" => Some(Self::Locked),
            "lock_io" => Some(Self::LockIo),
            "capacity" => Some(Self::Capacity),
            "limit" => Some(Self::Limit),
            "confirmation_required" => Some(Self::ConfirmationRequired),
            "snapshot_timeout" => Some(Self::SnapshotTimeout),
            "document_open" => Some(Self::DocumentOpen),
            "unsupported_document_open" => Some(Self::UnsupportedDocumentOpen),
            "xlsx_export" => Some(Self::XlsxExport),
            "FILE_SCOPE" => Some(Self::FileScope),
            "VAULT_LOCK" => Some(Self::VaultLock),
            "TASK_FAILED" => Some(Self::TaskFailed),
            _ => None,
        }
    }
}
impl From<ErrorCode> for String {
    fn from(code: ErrorCode) -> Self {
        code.as_str().into()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn known_code(&self) -> Option<ErrorCode> {
        ErrorCode::parse(&self.code)
    }
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl std::error::Error for AppError {}
pub type Result<T> = std::result::Result<T, AppError>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub workspace_name: String,
    pub display_currency: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatus {
    pub generation_id: String,
    pub data_dir: String,
    pub schema_version: u32,
    pub app_version: String,
    pub document_count: u64,
    pub mutation_sequence: u64,
    pub settings: Settings,
    pub country_count: u64,
    pub currency_count: u64,
    /// Activation completed, but a recovery journal still needs attention.
    #[serde(default)]
    pub recovery_required: bool,
    #[serde(default)]
    pub recovery_message: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DocumentSummary {
    pub id: String,
    pub original_name: String,
    pub mime_type: String,
    pub size_bytes: u64,
    pub sha256: String,
    pub added_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSummary {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub document_count: u64,
    pub blob_count: u64,
    pub created_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreInspection {
    pub token: String,
    pub archive_sha256: String,
    pub created_at: String,
    pub schema_version: u32,
    pub app_version: String,
    pub document_count: u64,
    pub blob_count: u64,
    pub settings: Settings,
    pub source_generation_id: String,
    pub current_generation_id: String,
    pub mutation_sequence: u64,
}
