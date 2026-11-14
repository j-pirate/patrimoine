//! Contrats du socle local. Les modules financiers seront ajoutés séparément.
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppError {
    pub code: String,
    pub message: String,
}
impl AppError {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
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
