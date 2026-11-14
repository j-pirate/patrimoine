//! Read-only OOXML: every source value is explicit text, never a formula.
use crate::{archives, db, files::*, portability, Vault};
use patrimoine_domain::{AppError, ExportSummary, Result};
use rusqlite::{types::ValueRef, Connection, Row};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    io::Write,
    path::Path,
};
use zip::{write::SimpleFileOptions, CompressionMethod, ZipWriter};

const MAX_ROWS: usize = 1_048_576;
const MAX_COLUMNS: usize = 16_384;
const MAX_CELL_UTF16: usize = 32_767;
const MAX_XML_BYTES: u64 = 512 * 1024 * 1024;
const XML_HEADER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const MAIN_NS: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

fn invalid_xlsx(message: &str) -> AppError {
    AppError::new("xlsx_export", message)
}
fn limit(message: &str) -> AppError {
    AppError::new("limit", message)
}

#[derive(Clone)]
struct Column {
    name: String,
    sqlite_type: String,
    nullable: bool,
    primary_key: usize,
    reference: String,
    convention: String,
}
#[derive(Clone)]
struct Table {
    name: String,
    columns: Vec<Column>,
}
struct Sheet {
    name: String,
}
struct Fragment {
    reference: String,
    table: String,
    key: String,
    column: String,
    number: usize,
    text: String,
}

fn xml_character(character: char) -> bool {
    matches!(character, '\t' | '\n' | '\r')
        || ('\u{20}'..='\u{d7ff}').contains(&character)
        || ('\u{e000}'..='\u{fffd}').contains(&character)
        || ('\u{10000}'..='\u{10ffff}').contains(&character)
}
fn excel_escape_at(value: &str, byte: usize) -> bool {
    let tail = &value.as_bytes()[byte..];
    tail.len() >= 7
        && tail[0] == b'_'
        && tail[1] == b'x'
        && tail[2..6].iter().all(u8::is_ascii_hexdigit)
        && tail[6] == b'_'
}
/// Protect OOXML's own escape notation as well as ordinary XML text.
fn escaped(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for (byte, character) in value.char_indices() {
        match character {
            '_' if excel_escape_at(value, byte) => out.push_str("_x005F_"),
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\r' => out.push_str("&#13;"),
            other => out.push(other),
        }
    }
    out
}

fn encoded_character(character: char) -> String {
    if !xml_character(character) {
        return format!("\\u{:04x}", character as u32);
    }
    match character {
        '"' => "\\\"".into(),
        '\\' => "\\\\".into(),
        '\n' => "\\n".into(),
        '\r' => "\\r".into(),
        '\t' => "\\t".into(),
        other => other.to_string(),
    }
}
/// Each fragment is one complete JSON string, decodable without this app.
fn fragments(value: &str, maximum: usize) -> Result<Vec<String>> {
    if maximum < 8 {
        return Err(limit(
            "La limite de cellule ne permet pas un fragment JSON.",
        ));
    }
    let mut result = Vec::new();
    let mut part = String::from("\"");
    let mut units = 2;
    for character in value.chars() {
        let encoded = encoded_character(character);
        let cost = encoded.encode_utf16().count();
        if units + cost > maximum && units > 2 {
            part.push('"');
            result.push(part);
            part = String::from("\"");
            units = 2;
        }
        part.push_str(&encoded);
        units += cost;
    }
    part.push('"');
    result.push(part);
    Ok(result)
}

fn column_name(mut index: usize) -> String {
    let mut letters = Vec::new();
    loop {
        letters.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    letters.into_iter().rev().collect()
}
fn short_name(value: &str, maximum: usize) -> String {
    let mut units = 0;
    value
        .chars()
        .take_while(|c| {
            units += c.len_utf16();
            units <= maximum
        })
        .collect()
}
fn french_table(name: &str) -> &str {
    match name {
        "account_reconciliations" => "Rapprochements banque",
        "app_settings" => "Réglages",
        "asset_ownerships" => "Propriétés actifs",
        "asset_profiles" => "Profils actifs",
        "assets" => "Actifs",
        "audit_events" => "Audit",
        "backup_runs" => "Journaux sauvegardes",
        "cash_links" => "Liens trésorerie",
        "change_sets" => "Changements",
        "company_distribution_cash_links" => "Liens dividendes",
        "company_distributions" => "Dividendes",
        "company_financial_periods" => "Indicateurs sociétés",
        "company_participations" => "Participations",
        "company_policies" => "Méthodes sociétés",
        "countries" => "Pays",
        "currencies" => "Devises",
        "debt_asset_links" => "Biens financés et sûretés",
        "debt_contracts" => "Financements",
        "debt_coverages" => "Couvertures sociétés",
        "debt_guarantees" => "Garanties",
        "debt_movements" => "Mouvements financement",
        "debt_parties" => "Parties financements",
        "debt_payment_allocations" => "Règlements financements",
        "debt_receivable_adjustments" => "Dépréciations créances",
        "debt_schedules" => "Échéances financements",
        "debt_terms" => "Conditions financements",
        "deposit_movements" => "Dépôts locatifs",
        "document_blobs" => "Originaux documentaires",
        "document_links" => "Associations documents",
        "document_versions" => "Versions documents",
        "documents" => "Documents",
        "entities" => "Entités",
        "entity_profiles" => "Profils entités",
        "exchange_rates" => "Taux de change",
        "family_memberships" => "Famille",
        "finance_coverages" => "Couverture bilan",
        "lease_terms" => "Conditions baux",
        "leases" => "Baux",
        "metal_cash_links" => "Liens trésorerie métaux",
        "metal_lots" => "Lots métaux",
        "metal_movements" => "Mouvements métaux",
        "metal_premiums" => "Primes métaux",
        "metal_quotes" => "Cours métaux",
        "objects" => "Identités objets",
        "profile_periods" => "Périodes profils",
        "property_details" => "Fiches immobilières",
        "property_events" => "Acquisitions et cessions",
        "rent_allocations" => "Affectations loyers",
        "rent_claim_adjustments" => "Dépréciations loyers",
        "rent_claim_parties" => "Créanciers loyers",
        "rent_claims" => "Créances loyers",
        "rent_dues" => "Échéances loyers",
        "rent_payments" => "Paiements loyers",
        "rental_expenses" => "Dépenses immobilières",
        "rental_units" => "Unités locatives",
        "revisions" => "Révisions",
        "schema_migrations" => "Schéma migrations",
        "treasury_accounts" => "Comptes et caisses",
        "treasury_entries" => "Lignes trésorerie",
        "treasury_transactions" => "Journal trésorerie",
        "valuations" => "Estimations",
        _ => name,
    }
}
fn convention(name: &str) -> String {
    if name.ends_with("__is_null") {
        return "1 = NULL ; 0 = valeur, y compris une chaîne réellement vide.".into();
    }
    if name.ends_with("_num") || name.ends_with("_den") {
        return "Entier textuel d’une fraction exacte ; numérateur et dénominateur restent séparés.".into();
    }
    if name.contains("currency") {
        return "Code de devise ISO ; aucune conversion implicite dans ce classeur.".into();
    }
    if name.ends_with("_at") {
        return "Instant ISO 8601 UTC, précision source conservée.".into();
    }
    if name.ends_with("_on")
        || name.contains("date")
        || name.starts_with("valid_")
        || name.starts_with("period_")
        || name == "tracking_start"
    {
        return "Date civile ISO 8601 ; intervalles [début, fin) lorsque deux bornes sont présentes.".into();
    }
    if name == "weight_unit" {
        return "g, kg ou troy_oz ; 1 once troy = 31,1034768 g exactement.".into();
    }
    if name == "purity" {
        return "Ratio décimal de métal fin, strictement positif et au plus égal à 1.".into();
    }
    if name.contains("quantity") {
        return "Quantité physique décimale exacte ; variation signée selon la nature du mouvement.".into();
    }
    if name.contains("amount")
        || name.contains("principal")
        || name.contains("cost")
        || name.contains("price")
        || name.contains("paid")
        || name == "rate"
        || name == "revenue"
        || name.contains("result")
        || name.contains("expenses")
        || name.contains("cash")
        || name.contains("receivables")
    {
        return "Valeur exacte conservée en texte ; devise/unité dans la ligne ou la source référencée. Ne pas convertir aveuglément en flottant.".into();
    }
    if name == "id" || name.ends_with("_id") {
        return "Identifiant stable ; relations explicites, versions et historiques conservés."
            .into();
    }
    "Valeur source conservée sans modification comme texte explicite ; données de gestion distinctes des totaux patrimoniaux.".into()
}
fn schema(c: &Connection) -> Result<Vec<Table>> {
    let mut query = c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(db::error)?;
    let names = query
        .query_map([], |r| r.get::<_, String>(0))
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error)?;
    let mut result = Vec::new();
    for name in names {
        let mut foreign_query = c
            .prepare(&format!("PRAGMA foreign_key_list(\"{name}\")"))
            .map_err(db::error)?;
        let foreign = foreign_query
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(3)?,
                    format!(
                        "{}.{}",
                        r.get::<_, String>(2)?,
                        r.get::<_, Option<String>>(4)?
                            .unwrap_or_else(|| "clé primaire".into())
                    ),
                ))
            })
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        let mut columns_query = c
            .prepare(&format!("PRAGMA table_info(\"{name}\")"))
            .map_err(db::error)?;
        let columns = columns_query
            .query_map([], |r| {
                let column: String = r.get(1)?;
                let notnull: bool = r.get(3)?;
                let pk: usize = r.get(5)?;
                Ok(Column {
                    name: column.clone(),
                    sqlite_type: r.get(2)?,
                    nullable: !notnull && pk == 0,
                    primary_key: pk,
                    reference: foreign
                        .iter()
                        .filter(|(from, _)| from == &column)
                        .map(|(_, to)| to.as_str())
                        .collect::<Vec<_>>()
                        .join(" ; "),
                    convention: convention(&column),
                })
            })
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        result.push(Table { name, columns });
    }
    Ok(result)
}
fn source_value(row: &Row<'_>, column: usize) -> Result<Option<String>> {
    match row.get_ref(column).map_err(db::error)? {
        ValueRef::Null => Ok(None),
        ValueRef::Integer(n) => Ok(Some(n.to_string())),
        ValueRef::Text(bytes) => Ok(Some(
            std::str::from_utf8(bytes)
                .map_err(|_| invalid_xlsx("Un texte source n’est pas un UTF-8 valide."))?
                .to_owned(),
        )),
        _ => Err(invalid_xlsx(
            "Un type source non textuel ou entier ne peut pas être exporté sans perte.",
        )),
    }
}
fn exported_columns(table: &Table) -> Vec<Column> {
    let mut result = Vec::new();
    for column in &table.columns {
        result.push(column.clone());
        if column.nullable {
            let name = format!("{}__is_null", column.name);
            result.push(Column {
                name: name.clone(),
                sqlite_type: "INTEGER (marqueur)".into(),
                nullable: false,
                primary_key: 0,
                reference: String::new(),
                convention: convention(&name),
            });
        }
    }
    result
}

struct Workbook {
    writer: ZipWriter<File>,
    sheets: Vec<Sheet>,
    names: BTreeSet<String>,
    dictionary: Vec<Vec<String>>,
    fragments: Vec<Fragment>,
    next_fragment: usize,
    bytes: u64,
    row_limit: usize,
}
impl Workbook {
    fn new(file: File, row_limit: usize) -> Result<Self> {
        if !(2..=MAX_ROWS).contains(&row_limit) {
            return Err(limit(
                "Une feuille doit pouvoir contenir son en-tête et au moins une ligne.",
            ));
        }
        Ok(Self {
            writer: ZipWriter::new(file),
            sheets: Vec::new(),
            names: BTreeSet::new(),
            dictionary: Vec::new(),
            fragments: Vec::new(),
            next_fragment: 0,
            bytes: 0,
            row_limit,
        })
    }
    fn write(&mut self, text: &str) -> Result<()> {
        self.bytes = self
            .bytes
            .checked_add(text.len() as u64)
            .ok_or_else(|| limit("Classeur trop volumineux."))?;
        if self.bytes > MAX_XML_BYTES {
            return Err(limit("Le classeur dépasse 512 Mio de données XML ; aucun fichier partiel n’a été publié."));
        }
        self.writer.write_all(text.as_bytes()).map_err(io)
    }
    fn entry(&mut self, name: &str) -> Result<()> {
        self.writer
            .start_file(
                name,
                SimpleFileOptions::default()
                    .compression_method(CompressionMethod::Deflated)
                    .unix_permissions(0o600),
            )
            .map_err(archives::zip_error)
    }
    fn name(&mut self, base: &str) -> Result<String> {
        if self.sheets.len() >= 9990 {
            return Err(limit(
                "Le classeur demanderait trop de feuilles ; aucune table n’a été tronquée.",
            ));
        }
        let safe = base.replace([':', '\\', '/', '?', '*', '[', ']'], "_");
        for number in 1..=10000 {
            let suffix = if number == 1 {
                String::new()
            } else {
                format!(" {number}")
            };
            let candidate = format!(
                "{}{}",
                short_name(safe.trim_matches('\''), 31 - suffix.encode_utf16().count()),
                suffix
            );
            if self.names.insert(candidate.to_lowercase()) {
                return Ok(candidate);
            }
        }
        Err(limit(
            "Les noms de feuilles ne peuvent pas être rendus uniques.",
        ))
    }
    fn start(&mut self, base: &str, table: &str, columns: &[Column]) -> Result<String> {
        if columns.len() > MAX_COLUMNS {
            return Err(limit(
                "Une table dépasse le nombre maximal de colonnes d’une feuille Excel.",
            ));
        }
        let name = self.name(base)?;
        let index = self.sheets.len() + 1;
        self.entry(&format!("xl/worksheets/sheet{index}.xml"))?;
        self.write(&format!("{XML_HEADER}<worksheet xmlns=\"{MAIN_NS}\"><sheetViews><sheetView workbookViewId=\"0\"><pane ySplit=\"1\" topLeftCell=\"A2\" activePane=\"bottomLeft\" state=\"frozen\"/></sheetView></sheetViews><sheetFormatPr defaultRowHeight=\"15\"/><cols><col min=\"1\" max=\"{}\" width=\"26\" customWidth=\"1\"/></cols><sheetData>",columns.len().max(1)))?;
        self.sheets.push(Sheet { name: name.clone() });
        self.row(
            1,
            &columns.iter().map(|c| c.name.clone()).collect::<Vec<_>>(),
            true,
        )?;
        for column in columns {
            self.dictionary.push(vec![
                name.clone(),
                table.into(),
                column.name.clone(),
                column.sqlite_type.clone(),
                if column.nullable { "oui" } else { "non" }.into(),
                if column.primary_key > 0 {
                    column.primary_key.to_string()
                } else {
                    String::new()
                },
                column.reference.clone(),
                column.convention.clone(),
            ]);
        }
        Ok(name)
    }
    fn end(&mut self, rows: usize, columns: usize) -> Result<()> {
        self.write(&format!("</sheetData><autoFilter ref=\"A1:{}{}\"/><pageMargins left=\"0.25\" right=\"0.25\" top=\"0.5\" bottom=\"0.5\" header=\"0.2\" footer=\"0.2\"/></worksheet>",column_name(columns.saturating_sub(1)),rows.max(1)))
    }
    fn row(&mut self, number: usize, values: &[String], header: bool) -> Result<()> {
        self.write(&format!("<row r=\"{number}\">"))?;
        for (column, value) in values.iter().enumerate() {
            if value.encode_utf16().count() > MAX_CELL_UTF16 || !value.chars().all(xml_character) {
                return Err(invalid_xlsx(
                    "Une cellule n’a pas été préparée pour le format XML/Excel.",
                ));
            }
            self.write(&format!("<c r=\"{}{number}\" t=\"inlineStr\" s=\"{}\"><is><t xml:space=\"preserve\">{}</t></is></c>",column_name(column),usize::from(header),escaped(value)))?;
        }
        self.write("</row>")
    }
    fn value(&mut self, table: &str, key: &str, column: &str, value: &str) -> Result<String> {
        // Calc normalizes CR/CRLF in ordinary text cells when saving again,
        // even when the original XML used a character reference for CR.
        // JSON fragments preserve these source characters across that round trip.
        if value.encode_utf16().count() <= MAX_CELL_UTF16
            && value.chars().all(xml_character)
            && !value.contains('\r')
        {
            return Ok(value.into());
        }
        self.next_fragment += 1;
        let reference = format!("TL{:08}", self.next_fragment);
        for (number, text) in fragments(value, MAX_CELL_UTF16)?.into_iter().enumerate() {
            self.fragments.push(Fragment {
                reference: reference.clone(),
                table: table.into(),
                key: key.into(),
                column: column.into(),
                number: number + 1,
                text,
            });
        }
        Ok(format!(
            "[Texte long {reference} : fragments JSON dans les feuilles Textes longs]"
        ))
    }
    fn table(&mut self, c: &Connection, table: &Table, query: &str, base: &str) -> Result<()> {
        let columns = exported_columns(table);
        self.start(base, &table.name, &columns)?;
        let mut row_number = 1usize;
        let mut source_number = 0usize;
        let mut statement = c.prepare(query).map_err(db::error)?;
        let mut rows = statement.query([]).map_err(db::error)?;
        while let Some(row) = rows.next().map_err(db::error)? {
            if row_number == self.row_limit {
                self.end(row_number, columns.len())?;
                self.start(base, &table.name, &columns)?;
                row_number = 1;
            }
            source_number += 1;
            let values = (0..table.columns.len())
                .map(|index| source_value(row, index))
                .collect::<Result<Vec<_>>>()?;
            let keys = table
                .columns
                .iter()
                .enumerate()
                .filter(|(_, col)| col.primary_key > 0)
                .map(|(index, col)| (col.name.clone(), values[index].clone()))
                .collect::<std::collections::BTreeMap<_, _>>();
            let key = serde_json::to_string(&keys)
                .map_err(|_| invalid_xlsx("Une clé de ligne ne peut pas être décrite."))?;
            let key = if key.encode_utf16().count() > 4096 || !key.chars().all(xml_character) {
                format!(
                    "ligne={source_number};clé_sha256={}",
                    hash_bytes(key.as_bytes())
                )
            } else {
                key
            };
            let mut exported = Vec::with_capacity(columns.len());
            for (column, value) in table.columns.iter().zip(values) {
                let was_null = value.is_none();
                exported.push(self.value(
                    &table.name,
                    &key,
                    &column.name,
                    value.as_deref().unwrap_or(""),
                )?);
                if column.nullable {
                    exported.push(if was_null { "1" } else { "0" }.into());
                }
            }
            row_number += 1;
            self.row(row_number, &exported, false)?;
        }
        self.end(row_number, columns.len())
    }
    fn simple(
        &mut self,
        base: &str,
        table: &str,
        headers: &[&str],
        rows: impl IntoIterator<Item = Vec<String>>,
    ) -> Result<()> {
        let columns = headers
            .iter()
            .map(|name| Column {
                name: (*name).into(),
                sqlite_type: "TEXT".into(),
                nullable: false,
                primary_key: 0,
                reference: String::new(),
                convention:
                    "Métadonnée de consultation ; hors des tables sources et des totaux financiers."
                        .into(),
            })
            .collect::<Vec<_>>();
        self.start(base, table, &columns)?;
        let mut number = 1;
        for row in rows {
            if number == self.row_limit {
                self.end(number, columns.len())?;
                self.start(base, table, &columns)?;
                number = 1;
            }
            let values = row
                .into_iter()
                .zip(headers)
                .map(|(value, column)| {
                    self.value(table, &format!("ligne={number}"), column, &value)
                })
                .collect::<Result<Vec<_>>>()?;
            number += 1;
            self.row(number, &values, false)?;
        }
        self.end(number, columns.len())
    }
    fn finish(mut self, created_at: &str) -> Result<()> {
        let mut fragment_rows = Vec::new();
        for fragment in std::mem::take(&mut self.fragments) {
            fragment_rows.push(vec![
                fragment.reference,
                fragment.table,
                fragment.key,
                fragment.column,
                fragment.number.to_string(),
                "JSON chaîne UTF-8".into(),
                fragment.text,
            ]);
        }
        self.simple(
            "Textes longs",
            "métadonnées_textes_longs",
            &[
                "Référence",
                "Table",
                "Clé primaire",
                "Colonne",
                "Fragment",
                "Encodage",
                "Texte",
            ],
            fragment_rows,
        )?;
        let dictionary = std::mem::take(&mut self.dictionary);
        self.simple(
            "Dictionnaire",
            "métadonnées_dictionnaire",
            &[
                "Feuille",
                "Table",
                "Colonne",
                "Type SQLite",
                "Nullable",
                "Clé primaire",
                "Référence",
                "Convention",
            ],
            dictionary,
        )?;
        self.entry("xl/workbook.xml")?;
        self.write(&format!("{XML_HEADER}<workbook xmlns=\"{MAIN_NS}\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><bookViews><workbookView/></bookViews><sheets>"))?;
        let names = self
            .sheets
            .iter()
            .map(|s| s.name.clone())
            .collect::<Vec<_>>();
        for (index, name) in names.iter().enumerate() {
            self.write(&format!(
                "<sheet name=\"{}\" sheetId=\"{}\" r:id=\"rId{}\"/>",
                escaped(name),
                index + 1,
                index + 1
            ))?;
        }
        self.write("</sheets></workbook>")?;
        self.entry("xl/_rels/workbook.xml.rels")?;
        self.write(&format!("{XML_HEADER}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">"))?;
        for index in 1..=self.sheets.len() {
            self.write(&format!("<Relationship Id=\"rId{index}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{index}.xml\"/>"))?;
        }
        self.write(&format!("<Relationship Id=\"rId{}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>",self.sheets.len()+1))?;
        self.entry("xl/styles.xml")?;
        self.write(&format!("{XML_HEADER}<styleSheet xmlns=\"{MAIN_NS}\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"2\"><xf numFmtId=\"49\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/><xf numFmtId=\"49\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\" applyFont=\"1\"/></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>"))?;
        self.entry("[Content_Types].xml")?;
        self.write(&format!("{XML_HEADER}<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/><Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>"))?;
        for index in 1..=self.sheets.len() {
            self.write(&format!("<Override PartName=\"/xl/worksheets/sheet{index}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"))?;
        }
        self.write("</Types>")?;
        self.entry("_rels/.rels")?;
        self.write(&format!("{XML_HEADER}<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/></Relationships>"))?;
        self.entry("docProps/core.xml")?;
        self.write(&format!("{XML_HEADER}<cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"><dc:creator>Patrimoine</dc:creator><dc:title>Export de consultation du coffre complet</dc:title><dcterms:created xsi:type=\"dcterms:W3CDTF\">{}</dcterms:created></cp:coreProperties>",escaped(created_at)))?;
        self.writer
            .finish()
            .map_err(archives::zip_error)?
            .sync_all()
            .map_err(io)
    }
}

fn information(
    c: &Connection,
    knowledge: u64,
    created: &str,
    count: usize,
) -> Result<Vec<Vec<String>>> {
    Ok(vec![
        vec!["Application".into(),"Patrimoine — classeur de consultation, sans réimportation XLSX implicite.".into()],
        vec!["Version application".into(),db::APP.into()], vec!["Schéma SQLite".into(),db::SCHEMA.to_string()],
        vec!["Création UTC".into(),created.into()], vec!["Connaissances".into(),knowledge.to_string()],
        vec!["Périmètre".into(),"Coffre complet : toutes les tables et versions, sans filtre membre/date. Les tables techniques et historiques ne sont pas des totaux à additionner.".into()],
        vec!["Espace".into(),db::settings(c)?.workspace_name], vec!["Tables sources".into(),count.to_string()],
        vec!["Valeurs exactes".into(),"Tous les nombres, identifiants et dates sont des cellules texte explicites. Aucune formule, macro, URL active ou conversion flottante n’est produite.".into()],
        vec!["NULL et vide".into(),"Une colonne nullable est suivie du marqueur __is_null : 1 pour NULL, 0 pour une valeur, y compris la chaîne vide. Une clé primaire est toujours renseignée.".into()],
        vec!["Dates et fractions".into(),"Dates civiles ISO 8601 ; instants UTC ; intervalles [début,fin). Numérateurs et dénominateurs exacts séparés ; devises et unités dans les sources.".into()],
        vec!["Métaux".into(),"Unités g, kg et troy_oz ; 1 kg = 1 000 g et 1 once troy = 31,1034768 g exactement. Les lots et mouvements sont des sources, pas un second stock à ajouter au bilan.".into()],
        vec!["Pièces originales".into(),"Ce classeur contient toutes les métadonnées et associations, pas les octets des fichiers. Utiliser la sauvegarde complète ou l’archive CSV structurée pour récupérer les originaux.".into()],
        vec!["Vues Banques et Caisses".into(),"Copies de consultation de treasury_accounts, toutes versions avec objet, séquence et annulation ; déjà présentes dans la table source. Ne pas les additionner une deuxième fois.".into()],
        vec!["Textes longs".into(),"Une cellule trop longue, contenant un caractère XML interdit ou un retour chariot renvoie à TLxxxxxxxx. L’encodage JSON préserve aussi les retours chariot lors d’un réenregistrement par un tableur. Trier ses fragments par numéro, décoder chaque cellule Texte comme chaîne JSON puis concaténer. Table, clé primaire et colonne identifient la source. Si la clé est trop longue, sa valeur source reste dans ses propres fragments et la métadonnée fournit sa ligne et son SHA-256.".into()],
        vec!["Échappement OOXML".into(),"Les motifs littéraux _xHHHH_ protègent leur soulignement selon OOXML (_x005F_). Les caractères XML interdits sont représentés par échappement JSON dans Textes longs, sans suppression.".into()],
        vec!["Limites de feuille".into(),"Les tables ou fragments dépassant 1 048 575 lignes de données sont répartis sur plusieurs feuilles avec en-tête répété, référencées au dictionnaire. Aucun texte ni ligne n’est tronqué.".into()],
    ])
}
fn write_workbook(
    c: &Connection,
    output: &Path,
    knowledge: u64,
    created: &str,
    row_limit: usize,
) -> Result<()> {
    let tables = schema(c)?;
    let mut workbook = Workbook::new(new_file(output)?, row_limit)?;
    workbook.simple(
        "Informations",
        "métadonnées_export",
        &["Champ", "Valeur"],
        information(c, knowledge, created, tables.len())?,
    )?;
    for table in &tables {
        let mut primary = table
            .columns
            .iter()
            .filter(|col| col.primary_key > 0)
            .collect::<Vec<_>>();
        primary.sort_by_key(|col| col.primary_key);
        let order = if primary.is_empty() {
            table
                .columns
                .iter()
                .map(|col| format!("\"{}\"", col.name))
                .collect::<Vec<_>>()
        } else {
            primary
                .iter()
                .map(|col| format!("\"{}\"", col.name))
                .collect::<Vec<_>>()
        };
        let query = format!(
            "SELECT {} FROM \"{}\" ORDER BY {}",
            table
                .columns
                .iter()
                .map(|col| format!("\"{}\"", col.name))
                .collect::<Vec<_>>()
                .join(","),
            table.name,
            order.join(",")
        );
        workbook.table(c, table, &query, french_table(&table.name))?;
    }
    if let Some(accounts) = tables.iter().find(|t| t.name == "treasury_accounts") {
        for (kind, label, name) in [
            ("bank", "Banques", "vue_banques"),
            ("cash", "Caisses", "vue_caisses"),
        ] {
            let mut columns = vec![
                Column {
                    name: "object_id".into(),
                    sqlite_type: "TEXT".into(),
                    nullable: false,
                    primary_key: 0,
                    reference: "objects.id".into(),
                    convention: convention("object_id"),
                },
                Column {
                    name: "knowledge_sequence".into(),
                    sqlite_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: 0,
                    reference: "change_sets.sequence".into(),
                    convention:
                        "Séquence de saisie/correction ; toutes les versions sont conservées."
                            .into(),
                },
                Column {
                    name: "is_void".into(),
                    sqlite_type: "INTEGER".into(),
                    nullable: false,
                    primary_key: 0,
                    reference: String::new(),
                    convention: "1 = révision annulée, 0 = source non annulée à cette révision."
                        .into(),
                },
            ];
            columns.extend(accounts.columns.clone());
            let table = Table {
                name: name.into(),
                columns,
            };
            let query=format!("SELECT r.object_id,cs.sequence,r.is_void,{} FROM treasury_accounts v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE v.kind='{kind}' ORDER BY r.object_id,cs.sequence",accounts.columns.iter().map(|col|format!("v.\"{}\"",col.name)).collect::<Vec<_>>().join(","));
            workbook.table(c, &table, &query, label)?;
        }
    }
    workbook.finish(created)
}

pub(crate) fn export_xlsx(vault: &Vault, target: &Path) -> Result<ExportSummary> {
    vault.ensure_active()?;
    if !target
        .extension()
        .and_then(|v| v.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("xlsx"))
    {
        return Err(AppError::new(
            "validation",
            "L’export de consultation doit avoir l’extension .xlsx.",
        ));
    }
    let parent = destination(target, &vault.root)?;
    let temporary = parent.join(format!(".patrimoine-{}.xlsx", id()));
    let scratch = vault.root.join("staging").join(id());
    let result = (|| {
        let snapshot = portability::capture_export_snapshot(vault, &scratch)?;
        let c = db::readonly(&snapshot.path)?;
        write_workbook(
            &c,
            &temporary,
            snapshot.knowledge_sequence,
            &now(),
            MAX_ROWS,
        )?;
        let (sha256, size_bytes) = hash_file(&temporary, MAX_XML_BYTES)?;
        publish_file(&temporary, target)?;
        Ok(ExportSummary {
            path: target.display().to_string(),
            sha256,
            size_bytes,
            format: "xlsx".into(),
            schema_version: db::SCHEMA,
            knowledge_sequence: snapshot.knowledge_sequence,
            table_count: snapshot.table_counts.len() as u64,
            row_count: snapshot.table_counts.values().sum(),
            document_count: *snapshot.table_counts.get("documents").unwrap_or(&0),
            blob_count: snapshot.blobs.len() as u64,
        })
    })();
    let _ = fs::remove_file(&temporary);
    archives::cleanup(&scratch);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use tempfile::TempDir;
    use zip::ZipArchive;

    fn xml(path: &Path, name: &str) -> String {
        let mut zip = ZipArchive::new(File::open(path).unwrap()).unwrap();
        let mut out = String::new();
        zip.by_name(name).unwrap().read_to_string(&mut out).unwrap();
        out
    }

    #[test]
    fn json_fragments_preserve_utf16_boundaries_and_xml_forbidden_characters() {
        let original = format!("{}\u{0001}\u{fffe}\r\n\t\\\"_x000A_", "é🙂".repeat(30));
        let parts = fragments(&original, 32).unwrap();
        assert!(parts.len() > 2);
        assert!(parts
            .iter()
            .all(|part| part.encode_utf16().count() <= 32 && part.chars().all(xml_character)));
        let restored = parts
            .iter()
            .map(|part| serde_json::from_str::<String>(part).unwrap())
            .collect::<String>();
        assert_eq!(restored, original);
    }
    #[test]
    fn long_source_is_replaced_by_a_recoverable_reference_without_silent_truncation() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("long.xlsx");
        let mut workbook = Workbook::new(new_file(&path).unwrap(), MAX_ROWS).unwrap();
        let original = "🙂".repeat(20_000);
        let marker = workbook
            .value("samples", "{\"id\":\"a\"}", "note", &original)
            .unwrap();
        assert!(marker.contains("TL00000001"));
        assert_eq!(workbook.fragments.len(), 2);
        assert!(workbook
            .fragments
            .iter()
            .all(|fragment| fragment.text.encode_utf16().count() <= MAX_CELL_UTF16));
        let restored = workbook
            .fragments
            .iter()
            .map(|fragment| serde_json::from_str::<String>(&fragment.text).unwrap())
            .collect::<String>();
        assert_eq!(restored, original);
        workbook
            .simple(
                "Exemple",
                "samples",
                &["id", "note"],
                vec![vec!["a".into(), marker]],
            )
            .unwrap();
        workbook.finish("2026-11-11T00:00:00Z").unwrap();
        assert!(xml(&path, "xl/worksheets/sheet1.xml").contains("TL00000001"));
        assert!(xml(&path, "xl/worksheets/sheet2.xml").contains("JSON chaîne UTF-8"));
    }
    #[test]
    fn row_overflow_creates_repeated_headers_and_dictionary_references() {
        let temp = TempDir::new().unwrap();
        let path = temp.path().join("rows.xlsx");
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE samples (id TEXT PRIMARY KEY, note TEXT);")
            .unwrap();
        for i in 0..8 {
            c.execute(
                "INSERT INTO samples VALUES(?1,?2)",
                [format!("{i:02}"), format!("row-{i}")],
            )
            .unwrap();
        }
        let mut workbook = Workbook::new(new_file(&path).unwrap(), 4).unwrap();
        let table = schema(&c).unwrap().remove(0);
        workbook
            .table(
                &c,
                &table,
                "SELECT id,note FROM samples ORDER BY id",
                "Exemples",
            )
            .unwrap();
        workbook.finish("2026-11-11T00:00:00Z").unwrap();
        let book = xml(&path, "xl/workbook.xml");
        assert!(book.contains("name=\"Exemples\""));
        assert!(book.contains("name=\"Exemples 2\""));
        assert!(book.contains("name=\"Exemples 3\""));
        let sheets = (1..=3)
            .map(|n| xml(&path, &format!("xl/worksheets/sheet{n}.xml")))
            .collect::<Vec<_>>();
        assert_eq!(
            sheets
                .iter()
                .map(|s| s.matches("<row ").count() - 1)
                .sum::<usize>(),
            8
        );
        assert!(sheets
            .iter()
            .all(|s| s.matches("<row ").count() <= 4 && s.contains(">note__is_null<")));
        assert!(sheets[0].contains("row-0") && sheets[2].contains("row-7"));
    }
    #[test]
    fn literal_ooxml_escape_patterns_are_not_decoded_as_control_data() {
        assert_eq!(escaped("_x000A_"), "_x005F_x000A_");
        assert_eq!(escaped("\r\n<&"), "&#13;\n&lt;&amp;");
        assert_eq!(column_name(16383), "XFD");
        assert_eq!(short_name(&"🙂".repeat(16), 31).encode_utf16().count(), 30);
    }
}
