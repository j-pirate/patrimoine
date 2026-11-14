#!/usr/bin/env python3
"""Reconstituer une archive CSV Patrimoine 1 / schéma 6 sans binaire applicatif.

Sous Linux, Python 3.10+ et SQLite 3.37+ suffisent. Les six migrations SQL locales du dépôt
sont vérifiées contre leurs empreintes épinglées ci-dessous. Aucun SQL provenant
du ZIP ou de son dictionnaire n'est exécuté. La sortie est une base indépendante,
jamais une activation du coffre : les vérifications SQLite, références, fichiers
et formats ne remplacent pas les validations financières du moteur Rust.

Exemple :
  python3 scripts/recover-csv.py export.zip --output recovered.sqlite3 \
    --documents-dir recovered-documents

Le dossier documentaire contient sha256/<préfixe>/<hash>. Dans les tables, le
chemin d'origine conserve documents/sha256/... ; retirer ce préfixe documents/
pour le résoudre depuis --documents-dir. Les sorties doivent être inexistantes.
"""

from __future__ import annotations

import argparse
import csv
from datetime import datetime, timedelta
import hashlib
import io
import json
import os
from pathlib import Path
import re
import shutil
import sqlite3
import stat
import sys
import tempfile
import uuid
import zipfile


SCHEMA_VERSION = 6
FORMAT_VERSION = 1
POLICY = "exact-rational-phase8-1"
MAX_FILE = 128 * 1024 * 1024
MAX_TOTAL = 512 * 1024 * 1024
MAX_MANIFEST = 4 * 1024 * 1024
MAX_ENTRIES = 11000
MAX_FIELD = 1024 * 1024
MAX_CELLS = 2_000_000
MIGRATIONS = (
    ("0001_foundation.sql", "10ffbddba9437cc39dedf8619f71a5afc4dd25a261696d72e47500ce500bc806"),
    ("0002_registry.sql", "1ad7c465039c94f421915252be5d57e58fb09825ea0c27e7edf61267df601d79"),
    ("0003_fx_history.sql", "385e47917de4bfe93a94507fd4f8a4eb97395ae0d881c7ca9e5a5e559657e2d4"),
    ("0004_property_rentals.sql", "dcfa5edb016ced531035d556df736bbb276ebbd95371b132fb7e79a1e9c8f701"),
    ("0005_finance.sql", "0db08b8d6b4f493f7689e3a9400b90ee6581687b0b9bdeb624898c9d5a891acd"),
    ("0006_metals_companies.sql", "64eaa09f5d58d1ac50b2ee730a4bc5f72d808348c838b9632e032372bcf1ffe2"),
)
DIALECT = {
    "encoding": "UTF-8 sans BOM",
    "delimiter": ",",
    "quote": '"',
    "line_endings": "CRLF",
    "null_markers": "<colonne>__is_null : 0=valeur, 1=NULL ; NULL exige une cellule vide",
}
MANIFEST_KEYS = {
    "archive_kind", "format_version", "schema_version", "app_version",
    "calculation_policy_version", "export_id", "created_at", "generation_id",
    "mutation_sequence", "document_count", "blob_count", "table_counts",
    "tables", "dialect", "files",
}
HASH = re.compile(r"[0-9a-f]{64}\Z")
INTEGER = re.compile(r"(?:0|-?[1-9][0-9]*)\Z")


class RecoveryError(Exception):
    """Une entrée ou une destination empêche une récupération complète."""


def require(condition, message):
    if not condition:
        raise RecoveryError(message)


def local_path(value):
    return Path(os.path.abspath(os.path.expanduser(os.fspath(value))))


def no_symlinks(path):
    cursor = Path(path.anchor)
    for part in path.parts[1:]:
        cursor /= part
        try:
            mode = cursor.lstat().st_mode
        except FileNotFoundError:
            continue
        require(not stat.S_ISLNK(mode), "Les chemins locaux ne doivent pas traverser de lien symbolique.")


def open_input(path):
    no_symlinks(path)
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        require(stat.S_ISREG(os.fstat(descriptor).st_mode), "L'entrée doit être un fichier ordinaire.")
        return os.fdopen(descriptor, "rb")
    except Exception:
        os.close(descriptor)
        raise


def new_file(path):
    descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0), 0o600)
    return os.fdopen(descriptor, "wb")


def stream_sha256(source):
    digest = hashlib.sha256()
    for chunk in iter(lambda: source.read(65536), b""):
        digest.update(chunk)
    return digest.hexdigest()


def duplicate_free_object(pairs):
    result = {}
    for key, value in pairs:
        require(key not in result, "Un objet JSON contient une clé dupliquée.")
        result[key] = value
    return result


def read_json(raw):
    def invalid_constant(_value):
        raise RecoveryError("Une constante JSON non standard est interdite.")
    return json.loads(raw.decode("utf-8"), object_pairs_hook=duplicate_free_object, parse_constant=invalid_constant)


def nonnegative_integer(value):
    return type(value) is int and value >= 0


def valid_uuid(value):
    try:
        return isinstance(value, str) and str(uuid.UUID(value)) == value
    except (ValueError, AttributeError):
        return False


def create_approved_schema(connection, schema_directory):
    no_symlinks(schema_directory)
    require(schema_directory.is_dir(), "Le dossier local des migrations SQL est absent.")
    connection.execute("PRAGMA trusted_schema=OFF")
    connection.execute("PRAGMA foreign_keys=OFF")
    connection.enable_load_extension(False)
    for name, expected in MIGRATIONS:
        with open_input(schema_directory / name) as source:
            raw = source.read(MAX_FILE + 1)
        require(hashlib.sha256(raw).hexdigest() == expected, f"La migration locale {name} ne correspond pas au SQL approuvé.")
        connection.executescript(raw.decode("utf-8"))
    connection.execute(f"PRAGMA user_version={SCHEMA_VERSION}")
    connection.execute("PRAGMA foreign_keys=ON")
    tables = []
    for (name,) in connection.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name"):
        columns = []
        for _, column, declared_type, not_null, _, primary_key in connection.execute(f'PRAGMA table_info("{name}")'):
            require(declared_type in ("TEXT", "INTEGER") and not column.endswith("__is_null"), "Le schéma local n'est pas pris en charge par le format CSV 1.")
            columns.append({"name": column, "declared_type": declared_type, "nullable": not bool(not_null) and primary_key == 0, "primary_key": primary_key})
        tables.append({"name": name, "columns": columns})
    require(len(tables) == 62, "Le schéma local ne contient pas les 62 tables attendues.")
    return tables


def headers(table):
    columns = table["columns"]
    return [column["name"] for column in columns] + [column["name"] + "__is_null" for column in columns if column["nullable"]]


def valid_table_metadata(value):
    if not isinstance(value, list):
        return False
    for table in value:
        if not isinstance(table, dict) or set(table) != {"name", "columns"} or not isinstance(table["name"], str) or not isinstance(table["columns"], list):
            return False
        for column in table["columns"]:
            if not isinstance(column, dict) or set(column) != {"name", "declared_type", "nullable", "primary_key"}:
                return False
            if not isinstance(column["name"], str) or not isinstance(column["declared_type"], str) or type(column["nullable"]) is not bool or not nonnegative_integer(column["primary_key"]):
                return False
    return True


def allowed_entry(name, table_names):
    if name in ("manifest.json", "dictionary/schema.json", "recovery/LISEZMOI.md"):
        return True
    if name in {f"tables/{table}.csv" for table in table_names}:
        return True
    parts = name.split("/")
    return len(parts) == 4 and parts[:2] == ["documents", "sha256"] and HASH.fullmatch(parts[3]) is not None and parts[2] == parts[3][:2]


def validate_manifest(manifest, tables, actual_names):
    require(isinstance(manifest, dict) and set(manifest) == MANIFEST_KEYS, "Le manifeste CSV est incomplet ou contient un champ inconnu.")
    require(manifest["archive_kind"] == "structured_csv" and type(manifest["format_version"]) is int and manifest["format_version"] == FORMAT_VERSION, "Le format d'archive CSV n'est pas pris en charge.")
    require(type(manifest["schema_version"]) is int and manifest["schema_version"] == SCHEMA_VERSION and manifest["calculation_policy_version"] == POLICY, "La version du schéma ou des règles n'est pas prise en charge.")
    require(isinstance(manifest["app_version"], str) and 0 < len(manifest["app_version"]) <= 128, "La version de l'application est invalide.")
    require(valid_uuid(manifest["export_id"]) and valid_uuid(manifest["generation_id"]), "Les identifiants du manifeste sont invalides.")
    require(isinstance(manifest["created_at"], str), "La date du manifeste est invalide.")
    instant = datetime.fromisoformat(manifest["created_at"].replace("Z", "+00:00"))
    require(instant.tzinfo is not None and instant.utcoffset() == timedelta(0), "La date du manifeste doit être en UTC.")
    require(all(nonnegative_integer(manifest[key]) for key in ("mutation_sequence", "document_count", "blob_count")), "Un compteur du manifeste est invalide.")
    require(valid_table_metadata(manifest["tables"]) and manifest["tables"] == tables and manifest["dialect"] == DIALECT, "Les colonnes ou le dialecte CSV diffèrent du format approuvé.")
    names = {table["name"] for table in tables}
    counts = manifest["table_counts"]
    require(isinstance(counts, dict) and set(counts) == names and all(nonnegative_integer(value) for value in counts.values()), "La liste des tables ou des nombres de lignes est invalide.")
    require(isinstance(manifest["files"], list), "La liste des fichiers est invalide.")
    declared = {"manifest.json"}
    for entry in manifest["files"]:
        require(isinstance(entry, dict) and set(entry) == {"path", "size_bytes", "sha256"}, "Une entrée du manifeste est invalide.")
        name = entry["path"]
        require(isinstance(name, str) and name not in declared and allowed_entry(name, names), "Un fichier du manifeste est inattendu ou dupliqué.")
        require(nonnegative_integer(entry["size_bytes"]) and entry["size_bytes"] <= MAX_FILE and isinstance(entry["sha256"], str) and HASH.fullmatch(entry["sha256"]) is not None, "Une taille ou empreinte du manifeste est invalide.")
        declared.add(name)
    required = {"manifest.json", "dictionary/schema.json", "recovery/LISEZMOI.md"} | {f"tables/{name}.csv" for name in names}
    require(required <= declared and declared == actual_names, "Un fichier manque ou le contenu ZIP diffère du manifeste.")


def extract_checked(archive, raw_directory, tables):
    table_names = {table["name"] for table in tables}
    require(0 < len(archive.infolist()) <= MAX_ENTRIES, "Le nombre de fichiers ZIP dépasse les limites.")
    actual = set()
    total = 0
    for item in archive.infolist():
        name = item.filename
        require(name == item.orig_filename and allowed_entry(name, table_names) and name not in actual, "Un chemin ZIP est interdit, inattendu ou dupliqué.")
        require(not item.is_dir() and stat.S_IFMT(item.external_attr >> 16) in (0, stat.S_IFREG), "Les liens, dossiers et fichiers spéciaux ZIP sont interdits.")
        require(not item.flag_bits & 1 and item.compress_type in (zipfile.ZIP_STORED, zipfile.ZIP_DEFLATED), "Le chiffrement ou la compression ZIP n'est pas pris en charge.")
        total += item.file_size
        require(item.file_size <= MAX_FILE and total <= MAX_TOTAL, "La taille décompressée du ZIP dépasse les limites.")
        require(item.file_size <= 1024 * 1024 or item.file_size <= item.compress_size * 200, "Le ratio de compression ZIP dépasse les limites.")
        actual.add(name)
    item = archive.getinfo("manifest.json")
    require(item.file_size <= MAX_MANIFEST, "Le manifeste dépasse 4 Mio.")
    with archive.open(item) as source:
        manifest = read_json(source.read(MAX_MANIFEST + 1))
    validate_manifest(manifest, tables, actual)
    for entry in manifest["files"]:
        item = archive.getinfo(entry["path"])
        require(item.file_size == entry["size_bytes"], "Une taille déclarée diffère du ZIP.")
        target = raw_directory / entry["path"]
        target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
        digest = hashlib.sha256()
        size = 0
        with archive.open(item) as source, new_file(target) as destination:
            while True:
                chunk = source.read(65536)
                if not chunk:
                    break
                size += len(chunk)
                require(size <= entry["size_bytes"], "Un fichier ZIP dépasse sa taille déclarée.")
                digest.update(chunk)
                destination.write(chunk)
        require(size == entry["size_bytes"] and digest.hexdigest() == entry["sha256"], "Une empreinte ou taille de fichier est incorrecte.")
    dictionary = read_json((raw_directory / "dictionary/schema.json").read_bytes())
    require(isinstance(dictionary, dict) and type(dictionary.get("csv_format_version")) is int and dictionary.get("csv_format_version") == FORMAT_VERSION and valid_table_metadata(dictionary.get("csv_tables")) and dictionary.get("csv_tables") == tables and dictionary.get("csv_dialect") == DIALECT, "Le dictionnaire CSV n'est pas cohérent avec le schéma approuvé.")
    # In particular, dictionary['tables'] may contain explanatory SQL text.
    # It is deliberately never passed to SQLite.
    return manifest


def check_csv_grammar(raw):
    require(not raw.startswith(b"\xef\xbb\xbf"), "Le CSV format 1 ne doit pas contenir de BOM.")
    raw.decode("utf-8")
    state = "start"
    field_size = 0
    position = 0
    while position < len(raw):
        byte = raw[position]
        if state == "quoted":
            if byte == 34:
                if position + 1 < len(raw) and raw[position + 1] == 34:
                    field_size += 1
                    position += 1
                else:
                    state = "closed"
            else:
                field_size += 1
        elif byte == 44:
            state, field_size = "start", 0
        elif byte == 13 and position + 1 < len(raw) and raw[position + 1] == 10:
            state, field_size = "start", 0
            position += 1
        elif state == "closed":
            raise RecoveryError("Un champ CSV fermé doit être suivi d'une virgule ou de CRLF.")
        elif byte == 34:
            require(state == "start", "Un guillemet CSV commence au milieu d'un champ.")
            state = "quoted"
        elif byte in (10, 13):
            raise RecoveryError("Les fins de ligne CSV doivent être CRLF.")
        else:
            state = "plain"
            field_size += 1
        require(field_size <= MAX_FIELD, "Une cellule CSV dépasse 1 Mio.")
        position += 1
    require(state != "quoted", "Un champ CSV n'est pas fermé.")


def import_order(connection, tables):
    """Use only the approved local schema; keep foreign keys enabled/deferred.

    Loading children first makes SQLite scan unindexed children for every
    later parent while resolving deferred constraints on large archives.
    """
    pending = {table['name']: table for table in tables}
    parents = {}
    for name in pending:
        quoted = '"' + name.replace('"', '""') + '"'
        parents[name] = {row[2] for row in connection.execute(f'PRAGMA foreign_key_list({quoted})') if row[2] != name}
        require(parents[name] <= pending.keys(), "Une référence du schéma local vise une table absente.")
    ordered, imported = [], set()
    while pending:
        ready = [name for name in sorted(pending) if parents[name] <= imported]
        require(ready, "Le schéma local contient un cycle de références entre tables.")
        for name in ready:
            ordered.append(pending.pop(name))
            imported.add(name)
    return ordered


def load_tables(connection, raw_directory, tables, manifest):
    old_limit = csv.field_size_limit(MAX_FIELD)
    try:
        connection.execute("BEGIN")
        connection.execute("PRAGMA defer_foreign_keys=ON")
        for table in import_order(connection, tables):
            name = table["name"]
            raw = (raw_directory / f"tables/{name}.csv").read_bytes()
            check_csv_grammar(raw)
            reader = csv.reader(io.StringIO(raw.decode("utf-8"), newline=""), strict=True)
            expected_headers = headers(table)
            require(next(reader, None) == expected_headers, f"Table {name}, ligne 1 : en-tête incorrect.")
            columns = table["columns"]
            sql = f'INSERT INTO "{name}" (' + ",".join('"' + column["name"] + '"' for column in columns) + ") VALUES(" + ",".join("?" for _ in columns) + ")"
            cells = len(expected_headers)
            row_count = 0
            for line, row in enumerate(reader, 2):
                require(len(row) == len(expected_headers), f"Table {name}, ligne {line} : nombre de cellules incorrect.")
                cells += len(row)
                require(cells <= MAX_CELLS, f"Table {name} : plus de deux millions de cellules.")
                nullable_index = len(columns)
                values = []
                for index, column in enumerate(columns):
                    context = f"Table {name}, ligne {line}, colonne {column['name']}"
                    is_null = False
                    if column["nullable"]:
                        flag = row[nullable_index]
                        nullable_index += 1
                        require(flag in ("0", "1"), context + " : marqueur NULL incorrect.")
                        is_null = flag == "1"
                    if is_null:
                        require(row[index] == "", context + " : un NULL ne peut pas porter une valeur.")
                        values.append(None)
                    elif column["declared_type"] == "INTEGER":
                        require(INTEGER.fullmatch(row[index]) is not None, context + " : entier non canonique.")
                        integer = int(row[index])
                        require(-(2 ** 63) <= integer < 2 ** 63, context + " : entier hors limites SQLite.")
                        values.append(integer)
                    else:
                        values.append(row[index])
                try:
                    connection.execute(sql, values)
                except sqlite3.Error as error:
                    raise RecoveryError(f"Table {name}, ligne {line} : {error}") from error
                row_count += 1
            require(row_count == manifest["table_counts"][name], f"Table {name} : le nombre de lignes diffère du manifeste.")
        require(not connection.execute("PRAGMA foreign_key_check").fetchall(), "Les références entre tables sont incohérentes.")
        connection.commit()
    except Exception:
        connection.rollback()
        raise
    finally:
        csv.field_size_limit(old_limit)


def validate_recovered(connection, raw_directory, manifest):
    require(connection.execute("PRAGMA integrity_check").fetchone()[0] == "ok", "L'intégrité SQLite est invalide.")
    require(not connection.execute("PRAGMA foreign_key_check").fetchall(), "Les références SQLite sont invalides.")
    require(connection.execute("SELECT version,checksum FROM schema_migrations ORDER BY version").fetchall() == [(index, checksum) for index, (_, checksum) in enumerate(MIGRATIONS, 1)], "Les métadonnées des migrations ne correspondent pas au schéma approuvé.")
    sequence = connection.execute("SELECT COALESCE(max(sequence),0) FROM change_sets").fetchone()[0]
    require(sequence == manifest["mutation_sequence"], "Le K source diffère du manifeste.")
    require(connection.execute("SELECT count(*) FROM documents").fetchone()[0] == manifest["document_count"] and connection.execute("SELECT count(*) FROM document_blobs").fetchone()[0] == manifest["blob_count"], "Les compteurs documentaires diffèrent du manifeste.")
    declared = {entry["path"] for entry in manifest["files"] if entry["path"].startswith("documents/")}
    required = set()
    for digest, size, relative in connection.execute("SELECT sha256,size_bytes,relative_path FROM document_blobs"):
        require(HASH.fullmatch(digest) is not None and relative == f"documents/sha256/{digest[:2]}/{digest}", "Un chemin documentaire est invalide.")
        path = raw_directory / relative
        require(path.is_file() and path.stat().st_size == size, "Un original documentaire manque ou sa taille est incorrecte.")
        with open_input(path) as source:
            require(stream_sha256(source) == digest, "L'empreinte d'un original documentaire est incorrecte.")
        required.add(relative)
    require(declared == required, "Les fichiers documentaires diffèrent des lignes document_blobs.")


def sync_directory(path):
    descriptor = os.open(path, os.O_RDONLY | getattr(os, "O_DIRECTORY", 0))
    try:
        os.fsync(descriptor)
    finally:
        os.close(descriptor)


def recover(archive, output, documents_dir, schema_directory=None):
    """Écrire deux sorties nouvelles après validation, sans changer le ZIP source."""
    archive, output, documents_dir = map(local_path, (archive, output, documents_dir))
    schema_directory = local_path(schema_directory or Path(__file__).parent.parent / "crates/infrastructure/migrations")
    documents_created = None
    output_created = False
    try:
        require(sqlite3.sqlite_version_info >= (3, 37, 0), "SQLite 3.37 ou plus récent est nécessaire pour les tables STRICT.")
        for destination in (output, documents_dir):
            no_symlinks(destination)
            require(not destination.exists() and not destination.is_symlink(), "Une destination existe déjà ; aucun remplacement n'est effectué.")
            require(destination.parent.is_dir(), "Les dossiers parents des sorties doivent déjà exister.")
        require(output != documents_dir, "Les deux destinations doivent être distinctes.")
        with tempfile.TemporaryDirectory(prefix=".recover-csv-", dir=output.parent) as directory:
            scratch = Path(directory)
            database = scratch / "recovered.sqlite3"
            with new_file(database):
                pass
            connection = sqlite3.connect(database)
            try:
                tables = create_approved_schema(connection, schema_directory)
                with open_input(archive) as source:
                    before = os.fstat(source.fileno())
                    require(before.st_size <= MAX_TOTAL, "Le ZIP source dépasse 512 Mio.")
                    archive_hash = stream_sha256(source)
                    source.seek(0)
                    with zipfile.ZipFile(source) as zipped:
                        raw_directory = scratch / "raw"
                        raw_directory.mkdir(mode=0o700)
                        manifest = extract_checked(zipped, raw_directory, tables)
                    after = os.fstat(source.fileno())
                    require((before.st_size, before.st_mtime_ns, before.st_ino) == (after.st_size, after.st_mtime_ns, after.st_ino), "Le ZIP source a changé pendant sa lecture.")
                load_tables(connection, raw_directory, tables, manifest)
                validate_recovered(connection, raw_directory, manifest)
            finally:
                connection.close()
            with open_input(database) as source:
                os.fsync(source.fileno())
            # Publish documents first. The SQLite file is linked exclusively last,
            # so a failed validation or pre-existing output never replaces data.
            documents_dir.mkdir(mode=0o700)
            documents_created = documents_dir.stat().st_ino
            for entry in manifest["files"]:
                if not entry["path"].startswith("documents/"):
                    continue
                target = documents_dir / entry["path"].removeprefix("documents/")
                target.parent.mkdir(mode=0o700, parents=True, exist_ok=True)
                with open_input(raw_directory / entry["path"]) as source, new_file(target) as destination:
                    shutil.copyfileobj(source, destination, length=65536)
                    destination.flush()
                    os.fsync(destination.fileno())
            for parent, _, _ in os.walk(documents_dir, topdown=False):
                sync_directory(parent)
            os.link(database, output, follow_symlinks=False)
            output_created = True
            sync_directory(output.parent)
            sync_directory(documents_dir.parent)
            return {
                "status": "recovered",
                "schema_version": SCHEMA_VERSION,
                "knowledge_sequence": manifest["mutation_sequence"],
                "table_count": len(tables),
                "row_count": sum(manifest["table_counts"].values()),
                "document_count": manifest["document_count"],
                "blob_count": manifest["blob_count"],
                "archive_sha256": archive_hash,
                "database": str(output),
                "documents_directory": str(documents_dir),
                "application_business_validation": False,
                "checks": ["approved local SQL", "manifest hashes", "CSV types and nulls", "SQLite integrity", "foreign keys", "original document hashes"],
            }
    except Exception as error:
        if not output_created and documents_created is not None:
            try:
                if not documents_dir.is_symlink() and documents_dir.stat().st_ino == documents_created:
                    shutil.rmtree(documents_dir)
            except OSError:
                pass
        if isinstance(error, RecoveryError):
            raise
        if isinstance(error, (OSError, ValueError, TypeError, KeyError, RecursionError, sqlite3.Error, csv.Error, zipfile.BadZipFile, zipfile.LargeZipFile)):
            raise RecoveryError(str(error)) from error
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("archive", type=Path, help="Archive ZIP CSV structurée, format 1 / schéma 6")
    parser.add_argument("--output", type=Path, required=True, help="Nouveau fichier SQLite, jamais remplacé")
    parser.add_argument("--documents-dir", type=Path, required=True, help="Nouveau dossier des originaux, contenant sha256/")
    parser.add_argument("--schema-directory", type=Path, help="Six migrations SQL locales approuvées ; défaut : celles du dépôt")
    arguments = parser.parse_args()
    try:
        result = recover(arguments.archive, arguments.output, arguments.documents_dir, arguments.schema_directory)
    except RecoveryError as error:
        print(f"Récupération refusée : {error}", file=sys.stderr)
        return 1
    print(json.dumps(result, ensure_ascii=False, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
