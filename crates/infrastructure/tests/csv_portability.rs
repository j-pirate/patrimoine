//! Portable archives are reconstructed, not treated as a trusted SQLite input.
use patrimoine_application::{DocumentService, PortabilityService, RegistryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use rusqlite::{types::ValueRef, Connection};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use zip::{write::SimpleFileOptions, CompressionMethod, ZipArchive, ZipWriter};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn active_db(root: &Path) -> PathBuf {
    root.join("generations")
        .join(fs::read_to_string(root.join("CURRENT")).unwrap().trim())
        .join("patrimoine.sqlite3")
}
fn content(root: &Path) -> BTreeMap<String, Vec<Value>> {
    let c = Connection::open(active_db(root)).unwrap();
    let names=c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap().query_map([],|r|r.get::<_,String>(0)).unwrap().collect::<std::result::Result<Vec<_>,_>>().unwrap();
    let mut out = BTreeMap::new();
    for name in names {
        let mut q = c.prepare(&format!("SELECT * FROM \"{name}\"")).unwrap();
        let count = q.column_count();
        let mut rows = q
            .query_map([], |row| {
                Ok(Value::Array(
                    (0..count)
                        .map(|i| match row.get_ref(i).unwrap() {
                            ValueRef::Null => Value::Null,
                            ValueRef::Integer(n) => json!(n),
                            ValueRef::Text(s) => json!(std::str::from_utf8(s).unwrap()),
                            _ => panic!("Unexpected SQL storage class"),
                        })
                        .collect(),
                ))
            })
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap();
        rows.sort_by_cached_key(|v| v.to_string());
        out.insert(name, rows);
    }
    out
}
fn fixture() -> (TempDir, Vault, Entity) {
    let temp = TempDir::new().unwrap();
    let mut v = Vault::open(temp.path().join("source")).unwrap();
    let owner = v
        .save_entity(SaveEntityInput {
            id: None,
            expected_revision_id: None,
            reason: "Sources exactes CSV".into(),
            kind: "person".into(),
            display_name: "Émilie CSV".into(),
            first_name: None,
            last_name: Some("CSV".into()),
            birth_date: None,
            residence_country: Some("FR".into()),
            preferred_currency: "EUR".into(),
            activity: String::new(),
            notes: "=HYPERLINK(\"https://example.invalid\");\n@note\nété,+001".into(),
            archived: false,
        })
        .unwrap();
    v.save_membership(SaveMembershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Membre CSV".into(),
        person_id: owner.id.clone(),
        valid_from: "2020-01-01".into(),
        valid_to: None,
        relationship: "Famille".into(),
    })
    .unwrap();
    let asset = v
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Actif CSV".into(),
            name: "Actif exact".into(),
            country: "FR".into(),
            category: "autre".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    v.save_ownership(SaveOwnershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Droit exact CSV".into(),
        asset_id: asset.id.clone(),
        owner_entity_id: owner.id.clone(),
        legal_share: Some(Fraction {
            num: "1".into(),
            den: "1".into(),
        }),
        economic_share: Some(Fraction {
            num: "1".into(),
            den: "3".into(),
        }),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    })
    .unwrap();
    v.save_valuation(SaveValuationInput {
        id: None,
        expected_revision_id: None,
        reason: "Montant maximum précis".into(),
        target_kind: "asset".into(),
        target_id: asset.id,
        amount: "9999999999999999.12345678".into(),
        currency: "EUR".into(),
        effective_on: "2020-01-01".into(),
        source: "Ne jamais convertir en flottant".into(),
    })
    .unwrap();
    let bytes = b"Original CSV\0\xff\n";
    for name in ["Pièce A.bin", "Pièce B.bin"] {
        let path = temp.path().join(name);
        fs::write(&path, bytes).unwrap();
        let document = v
            .import_document(
                &path,
                "Deux noms, un original physique\r\nDescription exacte, avec ligne\nété",
            )
            .unwrap();
        v.save_document_link(SaveDocumentLinkInput {
            id: None,
            expected_revision_id: None,
            reason: "Association CSV exacte".into(),
            document_id: document.id,
            target_object_id: owner.id.clone(),
            role: if name == "Pièce A.bin" {
                None
            } else {
                Some(String::new())
            },
            valid_from: Some("2020-01-01".into()),
            valid_to: None,
        })
        .unwrap();
    }
    (temp, v, owner)
}
fn zip_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut z = ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
    let mut entries = BTreeMap::new();
    for i in 0..z.len() {
        let mut f = z.by_index(i).unwrap();
        let name = f.name().unwrap().into_owned();
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes).unwrap();
        entries.insert(name, bytes);
    }
    entries
}
fn write_zip(path: &Path, entries: &BTreeMap<String, Vec<u8>>) {
    let mut z = ZipWriter::new(fs::File::create(path).unwrap());
    for (name, bytes) in entries {
        z.start_file(
            name,
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o600),
        )
        .unwrap();
        z.write_all(bytes).unwrap();
    }
    z.finish().unwrap();
}
fn rewrite(path: &Path, source: &Path, table: &str, from: &str, to: &str) {
    let mut entries = zip_files(source);
    let name = format!("tables/{table}.csv");
    let text = String::from_utf8(entries[&name].clone()).unwrap();
    assert!(text.contains(from), "Missing replacement {from:?}");
    let bytes = text.replacen(from, to, 1).into_bytes();
    entries.insert(name.clone(), bytes.clone());
    let mut manifest: Value = serde_json::from_slice(&entries["manifest.json"]).unwrap();
    let f = manifest["files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["path"] == name)
        .unwrap();
    f["sha256"] = hash(&bytes).into();
    f["size_bytes"] = (bytes.len() as u64).into();
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec_pretty(&manifest).unwrap(),
    );
    write_zip(path, &entries);
}

fn alter_central_directory(path: &Path, source: &Path, duplicate: bool) {
    let mut bytes = fs::read(source).unwrap();
    let end = bytes.len() - 22;
    assert_eq!(&bytes[end..end + 4], b"PK\x05\x06");
    let central = u32::from_le_bytes(bytes[end + 16..end + 20].try_into().unwrap()) as usize;
    let mut at = central;
    while at < end {
        assert_eq!(&bytes[at..at + 4], b"PK\x01\x02");
        let name_len = u16::from_le_bytes(bytes[at + 28..at + 30].try_into().unwrap()) as usize;
        let extra_len = u16::from_le_bytes(bytes[at + 30..at + 32].try_into().unwrap()) as usize;
        let comment_len = u16::from_le_bytes(bytes[at + 32..at + 34].try_into().unwrap()) as usize;
        let next = at + 46 + name_len + extra_len + comment_len;
        if &bytes[at + 46..at + 46 + name_len] == b"tables/entities.csv" {
            if duplicate {
                let record = bytes[at..next].to_vec();
                let mut ending = bytes[end..].to_vec();
                for offset in [8, 10] {
                    let count =
                        u16::from_le_bytes(ending[offset..offset + 2].try_into().unwrap()) + 1;
                    ending[offset..offset + 2].copy_from_slice(&count.to_le_bytes());
                }
                let size =
                    u32::from_le_bytes(ending[12..16].try_into().unwrap()) + record.len() as u32;
                ending[12..16].copy_from_slice(&size.to_le_bytes());
                bytes.truncate(end);
                bytes.extend(record);
                bytes.extend(ending);
            } else {
                bytes[at + 38..at + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
            }
            fs::write(path, bytes).unwrap();
            return;
        }
        at = next;
    }
    panic!("Central entry not found");
}
fn assert_only_import_provenance(
    before: &BTreeMap<String, Vec<Value>>,
    after: &BTreeMap<String, Vec<Value>>,
) {
    for (name, rows) in before {
        if name == "change_sets" || name == "audit_events" {
            assert_eq!(after[name].len(), rows.len() + 1, "{name}");
            assert!(rows.iter().all(|r| after[name].contains(r)), "{name}");
        } else {
            assert_eq!(rows, &after[name], "{name}");
        }
    }
    let added = after["audit_events"]
        .iter()
        .find(|r| !before["audit_events"].contains(r))
        .unwrap();
    assert_eq!(added[4], "csv_imported");
    assert_eq!(added[3], Value::Null);
}

#[test]
fn whole_archive_rebuilds_all_tables_exact_text_nulls_originals_and_old_knowledge() {
    let (temp, mut source, owner) = fixture();
    let before = content(&temp.path().join("source"));
    let sequence = source.status().unwrap().mutation_sequence;
    let path = temp.path().join("integral.zip");
    let summary = source.export_csv_archive(&path).unwrap();
    let archive_before = fs::read(&path).unwrap();
    assert_eq!(summary.table_count, 62);
    assert_eq!(summary.document_count, 2);
    assert_eq!(summary.blob_count, 1);
    assert_eq!(summary.knowledge_sequence, sequence);
    assert_eq!(source.status().unwrap().mutation_sequence, sequence);
    let files = zip_files(&path);
    let manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    assert_eq!(manifest["archive_kind"], "structured_csv");
    assert_eq!(
        files.keys().filter(|n| n.starts_with("tables/")).count(),
        62
    );
    let profiles = String::from_utf8(files["tables/entity_profiles.csv"].clone()).unwrap();
    assert!(profiles.contains("first_name__is_null"));
    assert!(profiles.contains("=HYPERLINK("));
    assert!(!profiles.starts_with('\u{feff}'));
    assert!(profiles.ends_with("\r\n"));
    assert!(String::from_utf8(files["tables/valuations.csv"].clone())
        .unwrap()
        .contains("9999999999999999.12345678"));
    let target_root = temp.path().join("target");
    let mut target = Vault::open(&target_root).unwrap();
    let old = target.status().unwrap();
    let inspection = target.inspect_csv_import(&path).unwrap();
    assert_eq!(target.status().unwrap().generation_id, old.generation_id);
    assert_eq!(
        target.status().unwrap().mutation_sequence,
        old.mutation_sequence
    );
    let restored = target
        .activate_restore(&inspection.token, true, None)
        .unwrap();
    assert_ne!(restored.generation_id, old.generation_id);
    assert_eq!(restored.mutation_sequence, sequence + 1);
    assert_only_import_provenance(&before, &content(&target_root));
    let loaded = target
        .list_registry_at(Some(sequence))
        .unwrap()
        .entities
        .into_iter()
        .find(|e| e.id == owner.id)
        .unwrap();
    assert_eq!(loaded.first_name, None);
    assert_eq!(loaded.last_name, Some("CSV".into()));
    let roles = target
        .list_document_library_at(Some(sequence))
        .unwrap()
        .links
        .into_iter()
        .map(|l| l.role)
        .collect::<Vec<_>>();
    assert!(roles.contains(&None));
    assert!(roles.contains(&Some(String::new())));
    assert_eq!(loaded.notes, owner.notes);
    assert_eq!(
        target.list_documents().unwrap(),
        source.list_documents().unwrap()
    );
    let copied = temp.path().join("original-copie.bin");
    target
        .export_document(&target.list_documents().unwrap()[0].id, &copied)
        .unwrap();
    assert_eq!(fs::read(copied).unwrap(), b"Original CSV\0\xff\n");
    assert!(target_root
        .join("generations")
        .join(old.generation_id)
        .exists());
    assert_eq!(fs::read(&path).unwrap(), archive_before);
}

#[test]
fn csv_without_documents_still_contains_empty_tables_and_reimports() {
    let temp = TempDir::new().unwrap();
    let mut v = Vault::open(temp.path().join("empty")).unwrap();
    let path = temp.path().join("empty.zip");
    v.export_csv_archive(&path).unwrap();
    let files = zip_files(&path);
    assert!(
        String::from_utf8(files["tables/metal_movements.csv"].clone())
            .unwrap()
            .ends_with("\r\n")
    );
    let mut other = Vault::open(temp.path().join("other")).unwrap();
    let inspected = other.inspect_csv_import(&path).unwrap();
    other
        .activate_restore(&inspected.token, true, None)
        .unwrap();
    assert!(other.list_documents().unwrap().is_empty());
}

#[test]
fn null_flags_unknown_headers_and_duplicate_keys_are_rejected_without_mutation() {
    let (temp, mut source, _) = fixture();
    let original = temp.path().join("original.zip");
    source.export_csv_archive(&original).unwrap();
    let target_root = temp.path().join("target");
    let mut target = Vault::open(&target_root).unwrap();
    let before = content(&target_root);
    let status = target.status().unwrap();
    let bad_header = temp.path().join("bad-header.zip");
    rewrite(
        &bad_header,
        &original,
        "entity_profiles",
        "\"first_name__is_null\"",
        "\"unknown__is_null\"",
    );
    assert!(target
        .inspect_csv_import(&bad_header)
        .unwrap_err()
        .message
        .contains("en-tête"));
    let null_invalid = temp.path().join("bad-null.zip");
    rewrite(
        &null_invalid,
        &original,
        "entity_profiles",
        "\"1\",\"0\",\"1\",\"0\"\r\n",
        "\"2\",\"0\",\"1\",\"0\"\r\n",
    );
    assert!(target.inspect_csv_import(&null_invalid).is_err());
    let wrong_name = temp.path().join("typed-int.zip");
    rewrite(
        &wrong_name,
        &original,
        "schema_migrations",
        "\"1\",\"foundation\"",
        "\"01\",\"foundation\"",
    );
    assert!(target
        .inspect_csv_import(&wrong_name)
        .unwrap_err()
        .message
        .contains("canonique"));
    assert_eq!(content(&target_root), before);
    assert_eq!(target.status().unwrap().generation_id, status.generation_id);
}

#[test]
fn recomputed_hashes_do_not_authorize_orphan_references_or_invalid_financial_values() {
    let (temp, mut source, owner) = fixture();
    let original = temp.path().join("source.zip");
    source.export_csv_archive(&original).unwrap();
    let mut target = Vault::open(temp.path().join("target")).unwrap();
    let orphan = temp.path().join("orphan.zip");
    rewrite(
        &orphan,
        &original,
        "family_memberships",
        &owner.id,
        "11111111-1111-4111-8111-111111111111",
    );
    assert!(target.inspect_csv_import(&orphan).is_err());
    let decimal = temp.path().join("decimal.zip");
    rewrite(
        &decimal,
        &original,
        "valuations",
        "9999999999999999.12345678",
        "1e3",
    );
    assert!(target.inspect_csv_import(&decimal).is_err());
    let dictionary = temp.path().join("dictionary.zip");
    let mut entries = zip_files(&original);
    let mut schema: Value = serde_json::from_slice(&entries["dictionary/schema.json"]).unwrap();
    schema["tables"]["objects"] = "DROP TABLE objects".into();
    let bytes = serde_json::to_vec(&schema).unwrap();
    entries.insert("dictionary/schema.json".into(), bytes.clone());
    let mut manifest: Value = serde_json::from_slice(&entries["manifest.json"]).unwrap();
    let f = manifest["files"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|f| f["path"] == "dictionary/schema.json")
        .unwrap();
    f["sha256"] = hash(&bytes).into();
    f["size_bytes"] = (bytes.len() as u64).into();
    entries.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    write_zip(&dictionary, &entries);
    assert!(target
        .inspect_csv_import(&dictionary)
        .unwrap_err()
        .message
        .contains("aucun SQL"));
}

#[test]
fn missing_original_future_versions_extra_files_and_path_traversal_are_rejected() {
    let (temp, mut source, _) = fixture();
    let original = temp.path().join("source.zip");
    source.export_csv_archive(&original).unwrap();
    let base = zip_files(&original);
    let mut target = Vault::open(temp.path().join("target")).unwrap();
    for (i, alteration) in [
        "missing-original",
        "future",
        "extra",
        "traversal",
        "bad-hash",
    ]
    .iter()
    .enumerate()
    {
        let mut entries = base.clone();
        match *alteration {
            "missing-original" => {
                let blob = entries
                    .keys()
                    .find(|n| n.starts_with("documents/"))
                    .unwrap()
                    .clone();
                entries.remove(&blob);
            }
            "future" => {
                let mut m: Value = serde_json::from_slice(&entries["manifest.json"]).unwrap();
                m["schema_version"] = 999.into();
                entries.insert("manifest.json".into(), serde_json::to_vec(&m).unwrap());
            }
            "extra" => {
                entries.insert("tables/unknown.csv".into(), b"x\r\n".to_vec());
            }
            "traversal" => {
                entries.insert("../outside".into(), b"outside".to_vec());
            }
            "bad-hash" => {
                let name = "tables/entities.csv";
                entries.get_mut(name).unwrap().push(b'X');
            }
            _ => unreachable!(),
        }
        let path = temp.path().join(format!("bad-{i}.zip"));
        write_zip(&path, &entries);
        assert!(target.inspect_csv_import(&path).is_err(), "{alteration}");
    }
    assert!(!temp.path().join("outside").exists());
}

#[test]
fn failed_exports_do_not_replace_existing_files_and_stale_import_tokens_are_rejected() {
    let (temp, mut source, _) = fixture();
    let path = temp.path().join("source.zip");
    source.export_csv_archive(&path).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(source.export_csv_archive(&path).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    let mut target = Vault::open(temp.path().join("target")).unwrap();
    let inspection = target.inspect_csv_import(&path).unwrap();
    target
        .save_settings(Settings {
            workspace_name: "Nouvelle mutation après analyse".into(),
            display_currency: "EUR".into(),
        })
        .unwrap();
    let status = target.status().unwrap();
    assert!(target
        .activate_restore(&inspection.token, true, None)
        .is_err());
    assert_eq!(target.status().unwrap().generation_id, status.generation_id);
    let inspection = target.inspect_csv_import(&path).unwrap();
    assert!(target
        .activate_restore(&inspection.token, false, None)
        .is_err());
    assert_eq!(target.status().unwrap().generation_id, status.generation_id);
}

#[test]
fn contradictory_null_duplicate_primary_keys_and_missing_even_empty_tables_are_rejected() {
    let (temp, mut source, _) = fixture();
    let original = temp.path().join("source.zip");
    source.export_csv_archive(&original).unwrap();
    let target_root = temp.path().join("target");
    let mut target = Vault::open(&target_root).unwrap();
    let before = content(&target_root);
    let status = target.status().unwrap();
    let null = temp.path().join("contradictory-null.zip");
    rewrite(
        &null,
        &original,
        "document_links",
        "\"\",\"2020-01-01\",\"\",\"1\",\"1\"\r\n",
        "\"Texte non vide\",\"2020-01-01\",\"\",\"1\",\"1\"\r\n",
    );
    assert!(target
        .inspect_csv_import(&null)
        .unwrap_err()
        .message
        .contains("NULL"));
    let duplicate = temp.path().join("duplicate-key.zip");
    rewrite(
        &duplicate,
        &original,
        "schema_migrations",
        "\"2\",\"registry\"",
        "\"1\",\"registry\"",
    );
    let error = target.inspect_csv_import(&duplicate).unwrap_err();
    assert!(error.message.contains("schema_migrations") && error.message.contains("UNIQUE"));
    let mut files = zip_files(&original);
    files.remove("tables/metal_premiums.csv");
    let mut manifest: Value = serde_json::from_slice(&files["manifest.json"]).unwrap();
    manifest["files"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f["path"] != "tables/metal_premiums.csv");
    files.insert(
        "manifest.json".into(),
        serde_json::to_vec(&manifest).unwrap(),
    );
    let missing = temp.path().join("missing-empty-table.zip");
    write_zip(&missing, &files);
    assert!(target
        .inspect_csv_import(&missing)
        .unwrap_err()
        .message
        .contains("table manque"));
    assert_eq!(content(&target_root), before);
    assert_eq!(target.status().unwrap().generation_id, status.generation_id);
    assert_eq!(
        target.status().unwrap().mutation_sequence,
        status.mutation_sequence
    );
}

#[test]
fn rehashed_csv_cannot_create_a_historical_ownership_cycle() {
    let (temp, mut source, owner) = fixture();
    let mut company = |name: &str| {
        source
            .save_entity(SaveEntityInput {
                id: None,
                expected_revision_id: None,
                reason: "Société synthétique CSV".into(),
                kind: "company".into(),
                display_name: name.into(),
                first_name: None,
                last_name: None,
                birth_date: None,
                residence_country: Some("FR".into()),
                preferred_currency: "EUR".into(),
                activity: String::new(),
                notes: String::new(),
                archived: false,
            })
            .unwrap()
    };
    let first = company("Société A CSV");
    let second = company("Société B CSV");
    for (company_id, owner_entity_id) in [(&first.id, &owner.id), (&second.id, &first.id)] {
        source
            .save_participation(SaveParticipationInput {
                id: None,
                expected_revision_id: None,
                reason: "Chaîne acyclique CSV".into(),
                company_id: company_id.clone(),
                owner_entity_id: owner_entity_id.clone(),
                legal_share: Some(Fraction {
                    num: "1".into(),
                    den: "1".into(),
                }),
                economic_share: Some(Fraction {
                    num: "1".into(),
                    den: "1".into(),
                }),
                voting_share: Some(Fraction {
                    num: "1".into(),
                    den: "1".into(),
                }),
                valid_from: "2020-01-01".into(),
                valid_to: None,
            })
            .unwrap();
    }
    let original = temp.path().join("source.zip");
    source.export_csv_archive(&original).unwrap();
    let bad = temp.path().join("historical-cycle.zip");
    rewrite(
        &bad,
        &original,
        "company_participations",
        &owner.id,
        &second.id,
    );
    let target_root = temp.path().join("target");
    let mut target = Vault::open(&target_root).unwrap();
    let before = content(&target_root);
    assert!(target
        .inspect_csv_import(&bad)
        .unwrap_err()
        .message
        .contains("cycle"));
    assert_eq!(content(&target_root), before);
}

#[test]
fn duplicate_raw_zip_entries_and_symlinks_are_rejected_before_extraction() {
    let (temp, mut source, _) = fixture();
    let original = temp.path().join("source.zip");
    source.export_csv_archive(&original).unwrap();
    let target_root = temp.path().join("target");
    let mut target = Vault::open(&target_root).unwrap();
    let before = content(&target_root);
    for (name, duplicate) in [("duplicate.zip", true), ("symlink.zip", false)] {
        let bad = temp.path().join(name);
        alter_central_directory(&bad, &original, duplicate);
        assert!(target.inspect_csv_import(&bad).is_err(), "{name}");
        assert_eq!(content(&target_root), before);
    }
}
