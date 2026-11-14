//! Whole-vault exports retain the snapshot and its recovery provenance.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExportSummary {
    pub path: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub format: String,
    pub schema_version: u32,
    pub knowledge_sequence: u64,
    pub table_count: u64,
    pub row_count: u64,
    pub document_count: u64,
    pub blob_count: u64,
}
