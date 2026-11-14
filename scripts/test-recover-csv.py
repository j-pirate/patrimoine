#!/usr/bin/env python3
"""Independent stdlib tests for the recovery CLI, using real schema-6 tables.

Run from the repository with ``python3 scripts/test-recover-csv.py``.
Fixtures are written with Python csv/sqlite3, without calling the exporter or
the recovery parser, and contain no real financial or documentary data.
"""

import copy
import csv
import hashlib
import importlib.util
import io
import json
import os
from pathlib import Path
import shutil
import sqlite3
import stat
import tempfile
import unittest
import uuid
import warnings
import zipfile


ROOT = Path(__file__).resolve().parents[1]
SCHEMA_DIR = ROOT / "crates/infrastructure/migrations"
STAMP = "2026-11-12T00:00:00Z"
POLICY = "exact-rational-phase8-1"
DIALECT = {
    "encoding": "UTF-8 sans BOM",
    "delimiter": ",",
    "quote": '"',
    "line_endings": "CRLF",
    "null_markers": "<colonne>__is_null : 0=valeur, 1=NULL ; NULL exige une cellule vide",
}
ORIGINAL = b"Original synth\xc3\xa9tique.\x00\xff\x10\n"
LITERAL_NOTE = '=HYPERLINK("https://example.invalid","Été")\r\n@note,+001,"quoted" 🌿'


def identifier(label):
    return str(uuid.uuid5(uuid.NAMESPACE_DNS, "patrimoine-recovery-test:" + label))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def sql_literal_identifier(name):
    return '"' + name.replace('"', '""') + '"'


def encode_rows(rows):
    stream = io.StringIO(newline="")
    writer = csv.writer(stream, delimiter=",", quotechar='"', doublequote=True,
                        lineterminator="\r\n", quoting=csv.QUOTE_ALL)
    writer.writerows(rows)
    return stream.getvalue().encode("utf-8")


def read_csv(data):
    return list(csv.reader(io.StringIO(data.decode("utf-8"), newline=""), strict=True))


def logical_rows(connection):
    tables = [row[0] for row in connection.execute(
        "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")]
    return {
        table: sorted(connection.execute(f"SELECT * FROM {sql_literal_identifier(table)}").fetchall(),
                      key=lambda row: json.dumps(row, ensure_ascii=False))
        for table in tables
    }


class CsvFixture:
    """A real SQLite schema and an independently generated format-1 archive."""

    def __init__(self, directory):
        self.directory = directory
        self.database = directory / "source.sqlite3"
        connection = sqlite3.connect(self.database)
        # Only the repository's local schema is executed, never archive DDL.
        migrations = sorted(SCHEMA_DIR.glob("000[1-6]_*.sql"))
        for migration in migrations:
            connection.executescript(migration.read_text())
        connection.execute("PRAGMA user_version=6")
        self.owner = identifier("owner")
        self.asset = identifier("asset")
        self.document_one = identifier("document-one")
        self.document_two = identifier("document-two")
        self.blob_hash = digest(ORIGINAL)
        self.blob_path = f"documents/sha256/{self.blob_hash[:2]}/{self.blob_hash}"
        workspace = identifier("workspace")
        for version, migration in enumerate(migrations, start=1):
            connection.execute("INSERT INTO schema_migrations VALUES(?,?,?,?,?)", (
                version, migration.stem.split("_", 1)[1], digest(migration.read_bytes()), STAMP, "0.1.0"))
        for country, name in [("FR", "France"), ("TN", "Tunisie"),
                              ("CH", "Suisse"), ("AE", "Émirats arabes unis")]:
            connection.execute("INSERT INTO countries VALUES(?,?)", (country, name))
        for code, name, minor in [("EUR", "Euro", 2), ("TND", "Dinar", 3),
                                  ("CHF", "Franc", 2), ("USD", "Dollar", 2),
                                  ("AED", "Dirham", 2)]:
            connection.execute("INSERT INTO currencies VALUES(?,?,?,1)", (code, name, minor))
        connection.execute("INSERT INTO objects VALUES(?,'workspace',?)", (workspace, STAMP))
        for key, value in [("workspace_id", workspace), ("workspace_name", "Récupération indépendante"),
                           ("display_currency", "EUR")]:
            connection.execute("INSERT INTO app_settings VALUES(?,?,'string',?)", (key, value, STAMP))
        for sequence in [1, 2, 3]:
            connection.execute("INSERT INTO change_sets VALUES(?,?,?,?,?)", (
                identifier(f"change-{sequence}"), sequence, STAMP,
                f"Source synthétique {sequence}", identifier(f"command-{sequence}")))

        def new_object(label, kind, sequence):
            object_id = identifier(label)
            revision = identifier(label + "-revision")
            connection.execute("INSERT INTO objects VALUES(?,?,?)", (object_id, kind, STAMP))
            connection.execute("INSERT INTO revisions VALUES(?,?,?,?,NULL,0,?)", (
                revision, object_id, identifier(f"change-{sequence}"), STAMP, "Création synthétique"))
            return object_id, revision

        owner, revision = new_object("owner", "entity", 1)
        connection.execute("INSERT INTO entities VALUES(?,'person')", (owner,))
        connection.execute("INSERT INTO entity_profiles VALUES(?,?,?,?,?,?,?,?,?,?,?)", (
            revision, owner, "Émilie", None, "", None, "FR", "EUR", "+001", LITERAL_NOTE, 0))
        # These SQL facts are independent of the application's DTO constructors.
        asset, revision = new_object("asset", "asset", 2)
        connection.execute("INSERT INTO assets VALUES(?)", (asset,))
        connection.execute("INSERT INTO asset_profiles VALUES(?,?,?,?,?,?,?)", (
            revision, asset, "Actif de précision", "FR", "autre", "", 0))
        _ownership, revision = new_object("ownership", "ownership", 2)
        connection.execute("INSERT INTO asset_ownerships VALUES(?,?,?,?,?,?,?,?,?)", (
            revision, asset, owner, "1", "3", "1", "3", "2024-01-01", None))
        _valuation, revision = new_object("valuation", "valuation", 2)
        connection.execute("INSERT INTO valuations VALUES(?,'asset',?,?,?,?,?)", (
            revision, asset, "9007199254740993.00000001", "EUR", "2024-01-01", "Décimal exact"))
        connection.execute("INSERT INTO document_blobs VALUES(?,?,?,?)", (
            self.blob_hash, len(ORIGINAL), self.blob_path, STAMP))
        for label in ["document-one", "document-two"]:
            document, revision = new_object(label, "document", 2)
            connection.execute("INSERT INTO documents VALUES(?)", (document,))
            connection.execute("INSERT INTO document_versions VALUES(?,?,?,?,?,'active')", (
                revision, self.blob_hash, "pièce.bin", "application/octet-stream", LITERAL_NOTE))
        # Logical deletion retains the prior version and shared original.
        connection.execute("INSERT INTO revisions VALUES(?,?,?,?,?,0,?)", (
            identifier("document-one-deleted"), self.document_one, identifier("change-3"), STAMP,
            identifier("document-one-revision"), "Suppression logique synthétique"))
        connection.execute("INSERT INTO document_versions VALUES(?,?,?,?,?,'deleted')", (
            identifier("document-one-deleted"), self.blob_hash, "pièce.bin", "application/octet-stream", ""))
        for label, target, role in [("link-null", owner, None), ("link-empty", asset, "")]:
            _link, revision = new_object(label, "document_link", 3)
            connection.execute("INSERT INTO document_links VALUES(?,?,?,?,?,NULL)", (
                revision, self.document_two, target, role, "2024-01-01"))
        connection.commit()
        connection.execute("PRAGMA foreign_keys=ON")
        if connection.execute("PRAGMA foreign_key_check").fetchall():
            raise AssertionError("The independent source fixture has foreign-key violations")
        self.expected = logical_rows(connection)
        self.tables = []
        self.payloads = {self.blob_path: ORIGINAL}
        self.counts = {}
        definitions = {}
        relations = {}
        for name in sorted(self.expected):
            quoted = sql_literal_identifier(name)
            definitions[name] = connection.execute("SELECT sql FROM sqlite_master WHERE name=?", (name,)).fetchone()[0]
            columns = [{"name": column[1], "declared_type": column[2],
                        "nullable": not bool(column[3]) and column[5] == 0,
                        "primary_key": column[5]}
                       for column in connection.execute(f"PRAGMA table_info({quoted})")]
            self.tables.append({"name": name, "columns": columns})
            headers = [column["name"] for column in columns]
            headers += [column["name"] + "__is_null" for column in columns if column["nullable"]]
            rows = []
            for record in self.expected[name]:
                values = ["" if value is None else str(value) for value in record]
                values += ["1" if record[index] is None else "0"
                           for index, column in enumerate(columns) if column["nullable"]]
                rows.append(values)
            self.counts[name] = len(rows)
            self.payloads[f"tables/{name}.csv"] = encode_rows([headers, *rows])
            relations[name] = [{"id": row[0], "sequence": row[1], "target_table": row[2],
                                "from": row[3], "to": row[4]}
                               for row in connection.execute(f"PRAGMA foreign_key_list({quoted})")]
        connection.close()
        self.payloads["dictionary/schema.json"] = json.dumps({
            "schema_version": 6, "tables": definitions,
            "csv_format_version": 1, "csv_tables": self.tables,
            "csv_dialect": DIALECT, "relations": relations,
            "conventions": {"numbers": "textes exacts"},
        }, ensure_ascii=False).encode()
        self.payloads["recovery/LISEZMOI.md"] = "Sources synthétiques, jamais de SQL d’archive à exécuter.\n".encode()
        self.manifest = {
            "archive_kind": "structured_csv", "format_version": 1, "schema_version": 6,
            "app_version": "0.1.0", "calculation_policy_version": POLICY,
            "export_id": identifier("export"), "created_at": STAMP,
            "generation_id": identifier("generation"), "mutation_sequence": 3,
            "document_count": 2, "blob_count": 1, "table_counts": self.counts,
            "tables": self.tables, "dialect": DIALECT, "files": [],
        }

    def archive(self, path, mutate=None, extra=None, duplicate=None):
        payloads = dict(self.payloads)
        manifest = copy.deepcopy(self.manifest)
        if mutate:
            mutate(manifest, payloads)
        manifest["files"] = [
            {"path": name, "size_bytes": len(data), "sha256": digest(data)}
            for name, data in sorted(payloads.items())
        ]
        entries = [("manifest.json", json.dumps(manifest, ensure_ascii=False).encode()), *payloads.items()]
        if extra:
            entries.extend(extra)
        if duplicate:
            entries.append((duplicate, payloads[duplicate]))
        with warnings.catch_warnings():
            warnings.simplefilter("ignore", UserWarning)
            with zipfile.ZipFile(path, "x", compression=zipfile.ZIP_STORED) as opened:
                for name, data in entries:
                    info = zipfile.ZipInfo(name)
                    info.create_system = 3
                    info.external_attr = (stat.S_IFREG | 0o600) << 16
                    opened.writestr(info, data)
        return path


class RecoveryTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        specification = importlib.util.spec_from_file_location("recovery_cli", ROOT / "scripts/recover-csv.py")
        cls.api = importlib.util.module_from_spec(specification)
        specification.loader.exec_module(cls.api)

    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="patrimoine-recovery-test-")
        self.directory = Path(self.temporary.name)
        self.fixture = CsvFixture(self.directory)
        self.output = self.directory / "recovered.sqlite3"
        self.documents = self.directory / "recovered-documents"

    def tearDown(self):
        self.temporary.cleanup()

    def rejected(self, archive, *, schema_directory=None):
        source_hash = digest(archive.read_bytes())
        with self.assertRaises(self.api.RecoveryError):
            self.api.recover(archive, self.output, self.documents, schema_directory)
        self.assertFalse(self.output.exists(), "A rejected source left a reconstructed database")
        self.assertFalse(self.documents.exists(), "A rejected source left partial originals")
        self.assertEqual(digest(archive.read_bytes()), source_hash)

    def test_local_import_order_covers_all_62_tables_with_parents_first(self):
        with sqlite3.connect(':memory:') as connection:
            tables = self.api.create_approved_schema(connection, SCHEMA_DIR)
            ordered = self.api.import_order(connection, tables)
            positions = {table['name']: index for index, table in enumerate(ordered)}
            self.assertEqual(set(positions), {table['name'] for table in tables})
            self.assertEqual(len(positions), 62)
            self.assertEqual(connection.execute('PRAGMA foreign_keys').fetchone()[0], 1)
            for table in tables:
                for row in connection.execute(f'PRAGMA foreign_key_list("{table["name"]}")'):
                    if row[2] != table['name']:
                        self.assertLess(positions[row[2]], positions[table['name']])
            self.assertLess(positions['revisions'], positions['audit_events'])
            self.assertLess(positions['documents'], positions['document_links'])

    def test_import_order_allows_self_reference_but_refuses_cross_table_cycle(self):
        with sqlite3.connect(':memory:') as connection:
            connection.execute('CREATE TABLE a(id TEXT PRIMARY KEY, parent TEXT REFERENCES a(id))')
            self.assertEqual([table['name'] for table in self.api.import_order(connection, [{'name': 'a'}])], ['a'])
            connection.execute('CREATE TABLE b(id TEXT PRIMARY KEY, parent TEXT REFERENCES c(id))')
            connection.execute('CREATE TABLE c(id TEXT PRIMARY KEY, parent TEXT REFERENCES b(id))')
            with self.assertRaises(self.api.RecoveryError):
                self.api.import_order(connection, [{'name': name} for name in ['a', 'b', 'c']])

    def test_all_62_tables_types_history_and_shared_original_are_recovered_exactly(self):
        archive = self.fixture.archive(self.directory / "source.zip")
        source_hash = digest(archive.read_bytes())
        result = self.api.recover(archive, self.output, self.documents)
        self.assertIsInstance(result, dict)
        with sqlite3.connect(self.output) as recovered:
            self.assertEqual(recovered.execute("PRAGMA user_version").fetchone()[0], 6)
            self.assertEqual(recovered.execute("PRAGMA integrity_check").fetchone()[0], "ok")
            self.assertFalse(recovered.execute("PRAGMA foreign_key_check").fetchall())
            actual = logical_rows(recovered)
            self.assertEqual(len(actual), 62)
            self.assertEqual(actual, self.fixture.expected)
            self.assertEqual(recovered.execute("SELECT max(sequence) FROM change_sets").fetchone()[0], 3)
            self.assertEqual(recovered.execute("SELECT count(*) FROM audit_events WHERE action='csv_imported'").fetchone()[0], 0)
            self.assertEqual(recovered.execute("SELECT first_name,last_name,activity,notes FROM entity_profiles").fetchone(),
                             (None, "", "+001", LITERAL_NOTE))
            self.assertEqual(recovered.execute("SELECT amount,typeof(amount) FROM valuations").fetchone(),
                             ("9007199254740993.00000001", "text"))
        blob = self.documents / "sha256" / self.fixture.blob_hash[:2] / self.fixture.blob_hash
        self.assertEqual(blob.read_bytes(), ORIGINAL)
        self.assertEqual(len([p for p in self.documents.rglob("*") if p.is_file()]), 1)
        self.assertEqual(digest(archive.read_bytes()), source_hash)

    def test_existing_database_and_existing_document_directory_are_never_overwritten(self):
        archive = self.fixture.archive(self.directory / "source.zip")
        self.output.write_bytes(b"An existing database must survive untouched")
        with self.assertRaises(self.api.RecoveryError):
            self.api.recover(archive, self.output, self.documents)
        self.assertEqual(self.output.read_bytes(), b"An existing database must survive untouched")
        self.assertFalse(self.documents.exists())
        self.output.unlink()
        self.documents.mkdir()
        sentinel = self.documents / "keep.txt"
        sentinel.write_text("Existing original")
        with self.assertRaises(self.api.RecoveryError):
            self.api.recover(archive, self.output, self.documents)
        self.assertFalse(self.output.exists())
        self.assertEqual(sentinel.read_text(), "Existing original")

    def test_manifest_over_four_mib_is_refused_without_partial_outputs(self):
        archive = self.fixture.archive(
            self.directory / "oversized-manifest.zip",
            mutate=lambda manifest, _: manifest.update({"padding": "x" * (4 * 1024 * 1024)}),
        )
        source_hash = digest(archive.read_bytes())
        with self.assertRaisesRegex(self.api.RecoveryError, "manifeste dépasse 4 Mio"):
            self.api.recover(archive, self.output, self.documents)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.documents.exists())
        self.assertEqual(digest(archive.read_bytes()), source_hash)

    def test_source_output_and_document_symlinks_are_rejected(self):
        archive = self.fixture.archive(self.directory / "source.zip")
        source_link = self.directory / "source-link.zip"
        source_link.symlink_to(archive)
        self.rejected(source_link)
        victim = self.directory / "victim.txt"
        victim.write_text("Keep")
        self.output.symlink_to(victim)
        with self.assertRaises(self.api.RecoveryError):
            self.api.recover(archive, self.output, self.documents)
        self.assertEqual(victim.read_text(), "Keep")
        self.output.unlink()
        real_documents = self.directory / "real-documents"
        real_documents.mkdir()
        self.documents.symlink_to(real_documents, target_is_directory=True)
        with self.assertRaises(self.api.RecoveryError):
            self.api.recover(archive, self.output, self.documents)
        self.assertFalse(self.output.exists())
        self.assertEqual(list(real_documents.iterdir()), [])

    def test_a_modified_local_migration_is_not_executed(self):
        archive = self.fixture.archive(self.directory / "source.zip")
        modified = self.directory / "untrusted-local-schema"
        shutil.copytree(SCHEMA_DIR, modified)
        sql = next(modified.glob("0001_*.sql"))
        sql.write_text(sql.read_text() + "\nCREATE TABLE unexpected(value TEXT);\n")
        self.rejected(archive, schema_directory=modified)

    def test_future_versions_and_policy_are_refused(self):
        for key, value in [("format_version", 2), ("schema_version", 7),
                           ("calculation_policy_version", "future-policy")]:
            with self.subTest(key=key):
                archive = self.fixture.archive(self.directory / f"future-{key}.zip",
                                               lambda manifest, _files: manifest.update({key: value}))
                self.rejected(archive)

    def test_boolean_numbers_and_integer_boolean_descriptors_are_not_coerced(self):
        changes = [
            ("boolean-format", lambda manifest: manifest.update(format_version=True)),
            ("boolean-count", lambda manifest: manifest.update(blob_count=True)),
            ("boolean-key", lambda manifest: manifest["tables"][0]["columns"][0].update(primary_key=True)),
            ("integer-nullable", lambda manifest: manifest["tables"][0]["columns"][0].update(nullable=0)),
        ]
        for label, change in changes:
            with self.subTest(label=label):
                archive = self.fixture.archive(self.directory / f"{label}.zip",
                                               lambda manifest, _files: change(manifest))
                self.rejected(archive)

    def test_zip_traversal_extra_files_and_duplicate_names_are_refused(self):
        for label, extra, duplicate in [
            ("traversal", [("../escaped.txt", b"No")], None),
            ("extra", [("tables/not_a_table.csv", b"\"value\"\r\n")], None),
            ("duplicate", None, "tables/countries.csv"),
        ]:
            with self.subTest(label=label):
                archive = self.fixture.archive(self.directory / f"{label}.zip", extra=extra, duplicate=duplicate)
                self.rejected(archive)
                self.assertFalse((self.directory.parent / "escaped.txt").exists())

    def test_a_symbolic_link_zip_entry_is_refused(self):
        archive = self.fixture.archive(self.directory / "symbolic-entry.zip")
        with zipfile.ZipFile(archive, "a") as opened:
            info = zipfile.ZipInfo("documents/sha256/aa/" + "a" * 64)
            info.create_system = 3
            info.external_attr = (stat.S_IFLNK | 0o777) << 16
            opened.writestr(info, "../../outside")
        self.rejected(archive)

    def test_unrehashed_table_damage_is_detected(self):
        archive = self.fixture.archive(self.directory / "bad-hash.zip")
        with zipfile.ZipFile(archive) as opened:
            entries = {info.filename: opened.read(info) for info in opened.infolist()}
        entries["tables/entity_profiles.csv"] += b"\r\n"
        replacement = self.directory / "rehoused.zip"
        with zipfile.ZipFile(replacement, "x") as opened:
            for name, data in entries.items():
                opened.writestr(name, data)
        self.rejected(replacement)

    def test_rehashed_null_marker_noncanonical_integer_header_and_foreign_key_are_refused(self):
        def changed_csv(files, table, column, value):
            rows = read_csv(files[f"tables/{table}.csv"])
            rows[1][rows[0].index(column)] = value
            files[f"tables/{table}.csv"] = encode_rows(rows)

        cases = [
            ("invalid-marker", "entity_profiles", "first_name__is_null", "2"),
            ("contradictory-null", "entity_profiles", "first_name", "Not null"),
            ("integer", "revisions", "is_void", "00"),
            ("orphan", "entities", "id", identifier("missing-object")),
        ]
        for label, table, column, value in cases:
            with self.subTest(label=label):
                archive = self.fixture.archive(self.directory / f"bad-{label}.zip",
                    lambda _m, files: changed_csv(files, table, column, value))
                self.rejected(archive)

        def unknown_header(_manifest, files):
            rows = read_csv(files["tables/countries.csv"])
            rows[0][0] = "unexpected_column"
            files["tables/countries.csv"] = encode_rows(rows)

        archive = self.fixture.archive(self.directory / "unknown-header.zip", unknown_header)
        self.rejected(archive)

    def test_rehashed_duplicate_primary_key_and_false_row_count_are_refused(self):
        def duplicate(manifest, files):
            rows = read_csv(files["tables/countries.csv"])
            rows.append(rows[1])
            files["tables/countries.csv"] = encode_rows(rows)
            manifest["table_counts"]["countries"] += 1

        self.rejected(self.fixture.archive(self.directory / "duplicate-key.zip", duplicate))
        self.rejected(self.fixture.archive(self.directory / "false-row-count.zip",
            lambda manifest, _files: manifest["table_counts"].update(countries=5)))

    def test_rehashed_missing_original_and_malformed_csv_are_refused(self):
        self.rejected(self.fixture.archive(self.directory / "missing-original.zip",
            lambda _manifest, files: files.pop(self.fixture.blob_path)))
        for label, invalid in [("bare-lf", b'"code","name_fr"\n"FR","France"\n'),
                               ("broken-quote", b'"code","name_fr"\r\n"FR","Never closed')]:
            with self.subTest(label=label):
                self.rejected(self.fixture.archive(self.directory / f"{label}.zip",
                    lambda _manifest, files: files.update({"tables/countries.csv": invalid})))

    def test_no_sql_in_the_archive_dictionary_is_executed(self):
        marker = self.directory / "injected.sqlite3"

        def poison_dictionary(_manifest, files):
            dictionary = json.loads(files["dictionary/schema.json"])
            dictionary["tables"]["objects"] = (
                f"ATTACH DATABASE '{marker}' AS injected; "
                "CREATE TABLE injected.unwanted(value TEXT); DETACH DATABASE injected; DROP TABLE countries;"
            )
            files["dictionary/schema.json"] = json.dumps(dictionary, ensure_ascii=False).encode()

        archive = self.fixture.archive(self.directory / "descriptive-ddl.zip", poison_dictionary)
        self.api.recover(archive, self.output, self.documents)
        self.assertFalse(marker.exists(), "Archive DDL had a filesystem side effect")
        with sqlite3.connect(self.output) as recovered:
            self.assertEqual(logical_rows(recovered), self.fixture.expected)


if __name__ == "__main__":
    unittest.main(verbosity=2)
