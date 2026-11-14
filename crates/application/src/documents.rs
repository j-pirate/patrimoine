use patrimoine_domain::*;
use std::path::Path;

pub trait DocumentService {
    fn list_document_library_at(
        &self,
        knowledge_sequence: Option<u64>,
    ) -> Result<DocumentLibrarySnapshot>;
    fn save_document_metadata(
        &mut self,
        input: SaveDocumentMetadataInput,
    ) -> Result<DocumentRecord>;
    fn replace_document(
        &mut self,
        input: ReplaceDocumentInput,
        source: &Path,
    ) -> Result<DocumentRecord>;
    fn save_document_link(&mut self, input: SaveDocumentLinkInput) -> Result<DocumentLink>;
    fn remove_document_link(&mut self, input: VoidRecordInput) -> Result<()>;
    fn export_document_revision(
        &self,
        document_id: &str,
        revision_id: &str,
        destination: &Path,
    ) -> Result<()>;
    fn prepare_document_open(
        &self,
        document_id: &str,
        revision_id: &str,
    ) -> Result<PreparedDocument>;
}
