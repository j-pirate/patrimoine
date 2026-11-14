export interface ExportSummary {
  path: string; sha256: string; size_bytes: number; format: string;
  schema_version: number; knowledge_sequence: number; table_count: number;
  row_count: number; document_count: number; blob_count: number;
}
