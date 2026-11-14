//! Read the exported OOXML independently: no production spreadsheet helpers.
use patrimoine_application::{PortabilityService, RegistryService, VaultService};
use patrimoine_domain::{SaveAssetInput, SaveEntityInput, SaveValuationInput};
use patrimoine_infrastructure::Vault;
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use zip::ZipArchive;

fn database(root: &Path) -> PathBuf {
    root.join("generations")
        .join(fs::read_to_string(root.join("CURRENT")).unwrap().trim())
        .join("patrimoine.sqlite3")
}

fn attribute(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let offset = tag.find(&key)? + key.len();
    Some(unescape(&tag[offset..offset + tag[offset..].find('"')?]))
}

fn unescape(value: &str) -> String {
    // XML 1.0 normalizes physical CR/CRLF before entity references are decoded.
    // Stored CR requires a character reference or a SpreadsheetML escape.
    let normalized = value.replace("\r\n", "\n").replace('\r', "\n");
    let mut output = String::new();
    let mut remaining = normalized.as_str();
    while let Some(index) = remaining.find('&') {
        output.push_str(&remaining[..index]);
        let entity = &remaining[index + 1..];
        let end = entity.find(';').expect("complete XML entity");
        let entity = &entity[..end];
        let character = match entity {
            "amp" => '&',
            "lt" => '<',
            "gt" => '>',
            "quot" => '"',
            "apos" => '\'',
            _ if entity.starts_with("#x") => {
                char::from_u32(u32::from_str_radix(&entity[2..], 16).unwrap()).unwrap()
            }
            _ if entity.starts_with('#') => char::from_u32(entity[1..].parse().unwrap()).unwrap(),
            _ => panic!("unexpected XML entity {entity}"),
        };
        output.push(character);
        remaining = &remaining[index + end + 2..];
    }
    output.push_str(remaining);
    output
}

fn column_index(reference: &str) -> usize {
    reference
        .bytes()
        .take_while(u8::is_ascii_alphabetic)
        .fold(0usize, |column, letter| {
            column * 26 + usize::from(letter.to_ascii_uppercase() - b'A') + 1
        })
        .checked_sub(1)
        .expect("cell column")
}

// SpreadsheetML escapes are decoded once, after XML entities. In particular
// `_x005F_x000A_` represents the literal text `_x000A_`, not a line break.
fn spreadsheet_text(value: &str) -> String {
    let mut output = String::new();
    let mut remaining = value;
    while !remaining.is_empty() {
        let bytes = remaining.as_bytes();
        if bytes.len() >= 7
            && bytes.starts_with(b"_x")
            && bytes[6] == b'_'
            && bytes[2..6].iter().all(u8::is_ascii_hexdigit)
        {
            let code = u32::from_str_radix(&remaining[2..6], 16).unwrap();
            output.push(char::from_u32(code).expect("valid scalar escape in fixture"));
            remaining = &remaining[7..];
        } else {
            let character = remaining.chars().next().unwrap();
            output.push(character);
            remaining = &remaining[character.len_utf8()..];
        }
    }
    output
}

fn rows(xml: &str) -> Vec<Vec<String>> {
    let mut output = Vec::new();
    for row in xml.split("<row").skip(1) {
        let row = row.split("</row>").next().unwrap();
        let mut cells = Vec::new();
        for cell in row.split("<c ").skip(1) {
            let end = cell.find("</c>").expect("closed spreadsheet cell");
            let cell = &cell[..end];
            let tag_end = cell.find('>').unwrap();
            let tag = &cell[..tag_end];
            assert_eq!(attribute(tag, "t").as_deref(), Some("inlineStr"));
            let reference = attribute(tag, "r").expect("explicit cell address");
            let index = column_index(&reference);
            let mut value = String::new();
            for text in cell[tag_end..].split("<t").skip(1) {
                let start = text.find('>').unwrap() + 1;
                let end = text.find("</t>").expect("closed cell text");
                value.push_str(&unescape(&text[start..end]));
            }
            if cells.len() <= index {
                cells.resize(index + 1, String::new());
            }
            cells[index] = spreadsheet_text(&value);
        }
        output.push(cells);
    }
    output
}

struct Workbook {
    files: BTreeMap<String, String>,
    sheets: BTreeMap<String, Vec<Vec<String>>>,
}
impl Workbook {
    fn open(path: &Path) -> Self {
        let mut archive = ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        let mut files = BTreeMap::new();
        for index in 0..archive.len() {
            let mut file = archive.by_index(index).unwrap();
            let mut contents = String::new();
            file.read_to_string(&mut contents).unwrap();
            assert!(files
                .insert(file.name().unwrap().into_owned(), contents)
                .is_none());
        }
        let mut relationships = BTreeMap::new();
        for fragment in files["xl/_rels/workbook.xml.rels"]
            .split("<Relationship ")
            .skip(1)
        {
            let tag = fragment.split('>').next().unwrap();
            relationships.insert(
                attribute(tag, "Id").unwrap(),
                attribute(tag, "Target").unwrap(),
            );
        }
        let mut sheets = BTreeMap::new();
        for fragment in files["xl/workbook.xml"].split("<sheet ").skip(1) {
            let tag = fragment.split('>').next().unwrap();
            let name = attribute(tag, "name").unwrap();
            assert!(name.chars().count() <= 31, "Excel sheet name: {name}");
            let relation = attribute(tag, "r:id").unwrap();
            let target = &relationships[&relation];
            let target = if target.starts_with('/') {
                target.trim_start_matches('/').into()
            } else {
                format!("xl/{target}")
            };
            assert!(
                sheets.insert(name.clone(), rows(&files[&target])).is_none(),
                "duplicate sheet {name}"
            );
        }
        Self { files, sheets }
    }

    fn table_sheet_names(&self, table: &str) -> BTreeSet<String> {
        let dictionary = &self.sheets["Dictionnaire"];
        dictionary
            .iter()
            .skip(1)
            .filter(|r| r.get(1).is_some_and(|v| v == table))
            .map(|r| r[0].clone())
            .collect()
    }

    fn table_rows(&self, table: &str) -> (Vec<String>, Vec<Vec<String>>) {
        let names = self.table_sheet_names(table);
        assert!(!names.is_empty(), "table not documented: {table}");
        let mut headers = None;
        let mut data = Vec::new();
        for name in names {
            let sheet = &self.sheets[&name];
            assert!(
                !sheet.is_empty(),
                "empty table still needs headers: {table}"
            );
            if let Some(expected) = &headers {
                assert_eq!(&sheet[0], expected);
            } else {
                headers = Some(sheet[0].clone());
            }
            data.extend_from_slice(&sheet[1..]);
        }
        (headers.unwrap(), data)
    }

    fn table_record(&self, table: &str, key: &str, value: &str) -> BTreeMap<String, String> {
        let (headers, data) = self.table_rows(table);
        let key = headers.iter().position(|column| column == key).unwrap();
        let row = data
            .iter()
            .find(|row| row.get(key).is_some_and(|cell| cell == value))
            .unwrap();
        headers
            .into_iter()
            .enumerate()
            .map(|(i, name)| (name, row.get(i).cloned().unwrap_or_default()))
            .collect()
    }
}

fn entity(name: &str) -> SaveEntityInput {
    SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Source documentaire synthétique".into(),
        kind: "person".into(),
        display_name: name.into(),
        first_name: None,
        last_name: Some(String::new()),
        birth_date: Some("1980-02-29".into()),
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: "+001234567890".into(),
        notes: "Été, € et 😀\nDeuxième ligne\tavec tabulation\nLittéraux _x000A_ et _x005F_".into(),
        archived: false,
    }
}

#[test]
fn workbook_contains_every_persistent_table_even_empty_and_matches_row_counts() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("vault");
    let mut vault = Vault::open(&root).unwrap();
    let connection = Connection::open(database(&root)).unwrap();
    let tables = connection.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").unwrap()
        .query_map([], |row| row.get::<_, String>(0)).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    let before = vault.status().unwrap();
    let output = temporary.path().join("complet.xlsx");
    let summary = vault.export_xlsx(&output).unwrap();
    assert_eq!(summary.format, "xlsx");
    assert_eq!(summary.schema_version, before.schema_version);
    assert_eq!(summary.knowledge_sequence, before.mutation_sequence);
    assert_eq!(summary.table_count, tables.len() as u64);
    assert_eq!(summary.document_count, 0);
    let bytes = fs::read(&output).unwrap();
    assert_eq!(summary.size_bytes, bytes.len() as u64);
    assert_eq!(summary.sha256, format!("{:x}", Sha256::digest(&bytes)));
    assert_eq!(summary.path, output.display().to_string());
    let workbook = Workbook::open(&output);
    assert!(workbook.sheets.contains_key("Informations"));
    assert!(workbook.sheets.contains_key("Dictionnaire"));
    let mut rows_count = 0;
    for table in &tables {
        let (headers, data) = workbook.table_rows(table);
        let source_count = connection
            .query_row(&format!("SELECT count(*) FROM \"{table}\""), [], |row| {
                row.get::<_, u64>(0)
            })
            .unwrap();
        assert_eq!(data.len() as u64, source_count, "table row count: {table}");
        rows_count += source_count;
        let source_columns = connection
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
            .unwrap()
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(3)?,
                    row.get::<_, u32>(5)?,
                ))
            })
            .unwrap()
            .collect::<rusqlite::Result<Vec<_>>>()
            .unwrap();
        for (column, not_null, primary_key) in source_columns {
            assert!(headers.contains(&column), "missing column {table}.{column}");
            if !not_null && primary_key == 0 {
                assert!(
                    headers.contains(&format!("{column}__is_null")),
                    "NULL flag {table}.{column}"
                );
            }
        }
    }
    assert_eq!(summary.row_count, rows_count);
    assert_eq!(
        vault.status().unwrap().mutation_sequence,
        before.mutation_sequence
    );
}

#[test]
fn formulas_urls_leading_zeroes_and_long_decimals_are_inert_exact_text_with_history_and_nulls() {
    let temporary = TempDir::new().unwrap();
    let mut vault = Vault::open(temporary.path().join("vault")).unwrap();
    let dangerous = "=HYPERLINK(\"https://example.invalid\",\"Émilie\")";
    let original = vault.save_entity(entity(dangerous)).unwrap();
    for name in ["+001234", "-003456", "@Somme"] {
        vault.save_entity(entity(name)).unwrap();
    }
    let mut correction = entity("Émilie corrigée");
    correction.id = Some(original.id.clone());
    correction.expected_revision_id = Some(original.revision_id.clone());
    let corrected = vault.save_entity(correction).unwrap();
    let asset = vault
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Actif synthétique".into(),
            name: "Précision".into(),
            country: "FR".into(),
            category: "autre".into(),
            notes: "https://example.invalid/aucun-lien-actif".into(),
            archived: false,
        })
        .unwrap();
    let valuation = vault
        .save_valuation(SaveValuationInput {
            id: None,
            expected_revision_id: None,
            reason: "Montant exact".into(),
            target_kind: "asset".into(),
            target_id: asset.id,
            amount: "12345678901234567890.1234".into(),
            currency: "EUR".into(),
            effective_on: "2025-01-01".into(),
            source: "@source".into(),
        })
        .unwrap();
    let output = temporary.path().join("texte.xlsx");
    vault.export_xlsx(&output).unwrap();
    let workbook = Workbook::open(&output);
    let old = workbook.table_record("entity_profiles", "revision_id", &original.revision_id);
    let new = workbook.table_record("entity_profiles", "revision_id", &corrected.revision_id);
    assert_eq!(old["display_name"], dangerous);
    assert_eq!(new["display_name"], "Émilie corrigée");
    assert_eq!(old["activity"], "+001234567890");
    assert_eq!(
        old["notes"],
        "Été, € et 😀\nDeuxième ligne\tavec tabulation\nLittéraux _x000A_ et _x005F_"
    );
    assert_eq!(old["birth_date"], "1980-02-29");
    assert_eq!(old["first_name"], "");
    assert_eq!(old["first_name__is_null"], "1");
    assert_eq!(old["last_name"], "");
    assert_eq!(old["last_name__is_null"], "0");
    let value = workbook.table_record("valuations", "revision_id", &valuation.revision_id);
    assert_eq!(value["amount"], "12345678901234567890.1234");
    assert_eq!(value["source"], "@source");
    for (name, xml) in &workbook.files {
        assert!(
            !name.contains("externalLinks") && !name.ends_with(".bin"),
            "active package part {name}"
        );
        assert!(
            !xml.contains("<f>") && !xml.contains("<f ") && !xml.contains("<hyperlink"),
            "active markup {name}"
        );
        if name.ends_with(".rels") {
            assert!(!xml.contains("TargetMode=\"External\""));
        }
    }
}

#[test]
fn document_metadata_are_exported_but_original_octets_stay_in_archives() {
    let temporary = TempDir::new().unwrap();
    let mut vault = Vault::open(temporary.path().join("vault")).unwrap();
    let original = temporary.path().join("=Facture-é.pdf");
    fs::write(
        &original,
        b"%PDF-original-binaire-synthetique-123456789\0\xff",
    )
    .unwrap();
    let description = "Une pièce à conserver\r\nDeuxième ligne\tavec tabulation";
    let document = vault.import_document(&original, description).unwrap();
    let output = temporary.path().join("documents.xlsx");
    let summary = vault.export_xlsx(&output).unwrap();
    assert_eq!(summary.document_count, 1);
    assert_eq!(summary.blob_count, 1);
    let workbook = Workbook::open(&output);
    let (_, versions) = workbook.table_rows("document_versions");
    assert!(versions
        .iter()
        .any(|row| row.iter().any(|value| value == "=Facture-é.pdf")));
    assert!(versions
        .iter()
        .any(|row| row.iter().any(|value| value == &document.sha256)));
    let revision = workbook.table_record("revisions", "object_id", &document.id);
    let version = workbook.table_record("document_versions", "revision_id", &revision["id"]);
    let marker = &version["description"];
    assert!(marker.starts_with("[Texte long TL"));
    // A short CRLF-bearing value also uses JSON: a normal cell loses CR when
    // LibreOffice saves the workbook again, despite XML character references.
    let fragments = &workbook.sheets["Textes longs"];
    let parts = fragments[1..]
        .iter()
        .filter(|row| row[1] == "document_versions" && row[3] == "description")
        .collect::<Vec<_>>();
    assert_eq!(parts.len(), 1);
    assert!(marker.contains(&parts[0][0]));
    assert!(parts[0][2].contains(&version["revision_id"]));
    assert_eq!(parts[0][5], "JSON chaîne UTF-8");
    assert!(parts[0][6].contains("\\r\\n"));
    assert!(!parts[0][6].contains('\r'));
    assert_eq!(
        serde_json::from_str::<String>(&parts[0][6]).unwrap(),
        description
    );
    assert!(!workbook
        .files
        .keys()
        .any(|name| name.starts_with("documents/")));
    let info = workbook.sheets["Informations"]
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(
        info.contains("originaux"),
        "originals are not embedded: notice required"
    );
}

#[test]
fn destination_failures_preserve_existing_file_and_live_vault() {
    let temporary = TempDir::new().unwrap();
    let root = temporary.path().join("vault");
    let mut vault = Vault::open(&root).unwrap();
    let before = vault.status().unwrap();
    let existing = temporary.path().join("existant.xlsx");
    fs::write(&existing, b"original a conserver").unwrap();
    assert!(vault.export_xlsx(&existing).is_err());
    assert_eq!(fs::read(&existing).unwrap(), b"original a conserver");
    assert!(vault.export_xlsx(&root.join("interdit.xlsx")).is_err());
    assert!(!root.join("interdit.xlsx").exists());
    let after = vault.status().unwrap();
    assert_eq!(before.generation_id, after.generation_id);
    assert_eq!(before.mutation_sequence, after.mutation_sequence);
}
