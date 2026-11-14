CREATE TABLE schema_migrations (
 version INTEGER PRIMARY KEY, name TEXT NOT NULL, checksum TEXT NOT NULL,
 applied_at TEXT NOT NULL, application_version TEXT NOT NULL
) STRICT;
CREATE TABLE countries (code TEXT PRIMARY KEY, name_fr TEXT NOT NULL) STRICT;
CREATE TABLE currencies (
 code TEXT PRIMARY KEY, name_fr TEXT NOT NULL, minor_unit INTEGER NOT NULL CHECK(minor_unit BETWEEN 0 AND 6),
 active INTEGER NOT NULL CHECK(active IN (0,1))
) STRICT;
CREATE TABLE objects (id TEXT PRIMARY KEY, kind TEXT NOT NULL CHECK(kind IN ('workspace','document','document_link')), created_at TEXT NOT NULL) STRICT;
CREATE TABLE change_sets (
 id TEXT PRIMARY KEY, sequence INTEGER NOT NULL UNIQUE CHECK(sequence>0), recorded_at TEXT NOT NULL,
 reason TEXT NOT NULL, command_id TEXT NOT NULL UNIQUE
) STRICT;
CREATE TABLE revisions (
 id TEXT PRIMARY KEY, object_id TEXT NOT NULL REFERENCES objects(id), change_set_id TEXT NOT NULL REFERENCES change_sets(id),
 recorded_at TEXT NOT NULL, supersedes_revision_id TEXT UNIQUE REFERENCES revisions(id),
 is_void INTEGER NOT NULL CHECK(is_void IN(0,1)), reason TEXT
) STRICT;
CREATE INDEX revisions_object ON revisions(object_id,recorded_at,id);
CREATE TABLE audit_events (
 id TEXT PRIMARY KEY, change_set_id TEXT NOT NULL REFERENCES change_sets(id), object_id TEXT NOT NULL REFERENCES objects(id),
 revision_id TEXT REFERENCES revisions(id), action TEXT NOT NULL, summary TEXT NOT NULL
) STRICT;
CREATE INDEX audit_object ON audit_events(object_id,change_set_id);
CREATE TABLE app_settings (key TEXT PRIMARY KEY, value_text TEXT NOT NULL, value_type TEXT NOT NULL, updated_at TEXT NOT NULL) STRICT;
CREATE TABLE document_blobs (
 sha256 TEXT PRIMARY KEY CHECK(length(sha256)=64), size_bytes INTEGER NOT NULL CHECK(size_bytes>=0),
 relative_path TEXT NOT NULL UNIQUE, created_at TEXT NOT NULL
) STRICT;
CREATE TABLE documents (id TEXT PRIMARY KEY REFERENCES objects(id)) STRICT;
CREATE TABLE document_versions (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), blob_sha256 TEXT NOT NULL REFERENCES document_blobs(sha256),
 original_name TEXT NOT NULL, mime_type TEXT NOT NULL, description TEXT NOT NULL,
 status TEXT NOT NULL CHECK(status IN('active','archived','deleted'))
) STRICT;
CREATE INDEX document_versions_blob ON document_versions(blob_sha256);
CREATE TABLE document_links (
 revision_id TEXT PRIMARY KEY REFERENCES revisions(id), document_id TEXT NOT NULL REFERENCES documents(id),
 target_object_id TEXT NOT NULL REFERENCES objects(id), role TEXT, valid_from TEXT NOT NULL, valid_to TEXT,
 CHECK(valid_to IS NULL OR valid_to>valid_from)
) STRICT;
CREATE INDEX document_links_target ON document_links(target_object_id,document_id);
CREATE TABLE backup_runs (
 id TEXT PRIMARY KEY, started_at TEXT NOT NULL, completed_at TEXT, kind TEXT NOT NULL,
 status TEXT NOT NULL, schema_version INTEGER NOT NULL, application_version TEXT NOT NULL,
 archive_sha256 TEXT, record_count INTEGER, failure_code TEXT
) STRICT;
