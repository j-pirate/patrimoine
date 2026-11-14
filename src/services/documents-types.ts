export interface DocumentRecord {
  id: string; revision_id: string; original_name: string; mime_type: string;
  description: string; status: string; sha256: string; size_bytes: number;
  added_at: string; recorded_at: string;
}
export interface DocumentLink {
  id: string; revision_id: string; document_id: string; target_object_id: string;
  role: string | null; valid_from: string | null; valid_to: string | null;
}
export interface DocumentTarget { id: string; kind: string; label: string; retired: boolean }
export interface DocumentLibrarySnapshot {
  documents: DocumentRecord[]; versions: DocumentRecord[]; links: DocumentLink[];
  targets: DocumentTarget[]; knowledge_sequence: number;
}
export interface SaveDocumentMetadataInput {
  id: string; expected_revision_id: string; reason: string;
  description: string; status: string; confirmed: boolean;
}
export interface ReplaceDocumentInput {
  id: string; expected_revision_id: string; reason: string; description: string;
}
export interface SaveDocumentLinkInput {
  id: string | null; expected_revision_id: string | null; reason: string;
  document_id: string; target_object_id: string; role: string | null;
  valid_from: string | null; valid_to: string | null;
}
export interface PreparedDocument {
  path: string; original_name: string; mime_type: string; sha256: string; size_bytes: number;
}
