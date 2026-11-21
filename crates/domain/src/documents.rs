//! Logical document versions and many-to-many associations retain their originals.
use serde::{Deserialize, Serialize};

/// Stable wire values; invalid persisted statuses never become arbitrary UI states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentStatus {
    Active,
    Archived,
    Deleted,
}
impl DocumentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Archived => "archived",
            Self::Deleted => "deleted",
        }
    }
}
impl TryFrom<&str> for DocumentStatus {
    type Error = crate::AppError;
    fn try_from(value: &str) -> crate::Result<Self> {
        match value {
            "active" => Ok(Self::Active),
            "archived" => Ok(Self::Archived),
            "deleted" => Ok(Self::Deleted),
            _ => Err(crate::AppError::new(
                "validation",
                "Le statut documentaire est inconnu.",
            )),
        }
    }
}
impl std::ops::Deref for DocumentStatus {
    type Target = str;
    fn deref(&self) -> &str {
        self.as_str()
    }
}
impl PartialEq<&str> for DocumentStatus {
    fn eq(&self, other: &&str) -> bool {
        self.as_str() == *other
    }
}
impl std::fmt::Display for DocumentStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentRecord {
    pub id: String,
    pub revision_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub description: String,
    pub status: DocumentStatus,
    pub sha256: String,
    pub size_bytes: u64,
    pub added_at: String,
    pub recorded_at: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentLink {
    pub id: String,
    pub revision_id: String,
    pub document_id: String,
    pub target_object_id: String,
    pub role: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DocumentTarget {
    pub id: String,
    pub kind: String,
    pub label: String,
    pub retired: bool,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentLibrarySnapshot {
    pub documents: Vec<DocumentRecord>,
    pub versions: Vec<DocumentRecord>,
    pub links: Vec<DocumentLink>,
    pub targets: Vec<DocumentTarget>,
    pub knowledge_sequence: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveDocumentMetadataInput {
    pub id: String,
    pub expected_revision_id: String,
    pub reason: String,
    pub description: String,
    pub status: String,
    pub confirmed: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplaceDocumentInput {
    pub id: String,
    pub expected_revision_id: String,
    pub reason: String,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SaveDocumentLinkInput {
    pub id: Option<String>,
    pub expected_revision_id: Option<String>,
    pub reason: String,
    pub document_id: String,
    pub target_object_id: String,
    pub role: Option<String>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PreparedDocument {
    pub path: String,
    pub original_name: String,
    pub mime_type: String,
    pub sha256: String,
    pub size_bytes: u64,
}
