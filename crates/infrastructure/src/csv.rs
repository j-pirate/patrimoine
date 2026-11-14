//! Lossless CSV cells and schema-derived nullable markers; no numeric coercion.
use crate::{db, files::*};
use patrimoine_domain::{AppError, Result};
use rusqlite::{types::ValueRef, Connection};
use serde::{Deserialize, Serialize};
use std::io::Write;

const MAX_CELLS: usize = 2_000_000;
const MAX_FIELD: usize = 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Column {
    pub name: String,
    pub declared_type: String,
    pub nullable: bool,
    pub primary_key: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct Table {
    pub name: String,
    pub columns: Vec<Column>,
}
impl Table {
    pub(crate) fn headers(&self) -> Vec<String> {
        self.columns
            .iter()
            .map(|c| c.name.clone())
            .chain(
                self.columns
                    .iter()
                    .filter(|c| c.nullable)
                    .map(|c| format!("{}__is_null", c.name)),
            )
            .collect()
    }
}

pub(crate) fn tables(c: &Connection) -> Result<Vec<Table>> {
    let names = c.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(db::error)?
        .query_map([], |r| r.get::<_, String>(0)).map_err(db::error)?.collect::<std::result::Result<Vec<_>, _>>().map_err(db::error)?;
    let mut result = Vec::new();
    for name in names {
        let columns = c
            .prepare(&format!("PRAGMA table_info(\"{name}\")"))
            .map_err(db::error)?
            .query_map([], |r| {
                let pk: u32 = r.get(5)?;
                Ok(Column {
                    name: r.get(1)?,
                    declared_type: r.get(2)?,
                    nullable: !r.get::<_, bool>(3)? && pk == 0,
                    primary_key: pk,
                })
            })
            .map_err(db::error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db::error)?;
        if columns.iter().any(|c| {
            !["TEXT", "INTEGER"].contains(&c.declared_type.as_str())
                || c.name.ends_with("__is_null")
        }) {
            return Err(invalid("Le schéma contient un type ou nom de colonne non pris en charge par le format CSV 1."));
        }
        result.push(Table { name, columns });
    }
    Ok(result)
}

pub(crate) fn record(writer: &mut impl Write, fields: &[String]) -> std::io::Result<()> {
    for (i, field) in fields.iter().enumerate() {
        if i > 0 {
            writer.write_all(b",")?;
        }
        // Quoting changes only the container syntax, never the source text.
        writer.write_all(b"\"")?;
        for part in field.split_inclusive('"') {
            writer.write_all(part.as_bytes())?;
            if part.ends_with('"') {
                writer.write_all(b"\"")?;
            }
        }
        writer.write_all(b"\"")?;
    }
    writer.write_all(b"\r\n")
}

pub(crate) fn write_table(c: &Connection, table: &Table, target: &std::path::Path) -> Result<()> {
    let mut writer = new_file(target)?;
    record(&mut writer, &table.headers()).map_err(io)?;
    let mut key = table
        .columns
        .iter()
        .filter(|c| c.primary_key > 0)
        .collect::<Vec<_>>();
    key.sort_by_key(|c| c.primary_key);
    let order = if key.is_empty() {
        table
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect::<Vec<_>>()
    } else {
        key.iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect::<Vec<_>>()
    };
    let mut query = c
        .prepare(&format!(
            "SELECT * FROM \"{}\" ORDER BY {}",
            table.name,
            order.join(",")
        ))
        .map_err(db::error)?;
    let mut rows = query.query([]).map_err(db::error)?;
    let mut cells = table.headers().len();
    while let Some(row) = rows.next().map_err(db::error)? {
        let mut fields = Vec::with_capacity(table.headers().len());
        let mut nulls = Vec::new();
        for (index, column) in table.columns.iter().enumerate() {
            let value = row.get_ref(index).map_err(db::error)?;
            let text = match value {
                ValueRef::Null => String::new(),
                ValueRef::Integer(n) if column.declared_type == "INTEGER" => n.to_string(),
                ValueRef::Text(s) if column.declared_type == "TEXT" => std::str::from_utf8(s)
                    .map_err(|_| invalid("Un texte SQLite n’est pas en UTF-8."))?
                    .to_owned(),
                _ => {
                    return Err(invalid(
                        "Une valeur SQLite ne correspond pas au type documenté dans le CSV.",
                    ))
                }
            };
            if text.len() > MAX_FIELD {
                return Err(invalid(
                    "Une cellule dépasse la limite CSV de 1 Mio ; aucune donnée n’a été tronquée.",
                ));
            }
            fields.push(text);
            if column.nullable {
                nulls.push(
                    if matches!(value, ValueRef::Null) {
                        "1"
                    } else {
                        "0"
                    }
                    .to_owned(),
                );
            }
        }
        fields.extend(nulls);
        cells += fields.len();
        if cells > MAX_CELLS {
            return Err(invalid("Une table dépasse la limite CSV de deux millions de cellules ; l’export est abandonné sans troncature."));
        }
        record(&mut writer, &fields).map_err(io)?;
        if writer.metadata().map_err(io)?.len() > crate::archives::MAX_FILE {
            return Err(invalid("Une table CSV dépasse 128 Mio."));
        }
    }
    writer.sync_all().map_err(io)
}

pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<Vec<String>>> {
    std::str::from_utf8(bytes).map_err(|_| invalid("Le CSV doit être en UTF-8."))?;
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(invalid("Le CSV format 1 ne doit pas contenir de BOM."));
    }
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = Vec::new();
    let mut state = 0u8;
    let mut at = 0usize;
    let mut cells = 0usize;
    let mut active = false;
    fn finish(field: &mut Vec<u8>, row: &mut Vec<String>, cells: &mut usize) -> Result<()> {
        *cells += 1;
        if *cells > MAX_CELLS {
            return Err(invalid("Le CSV dépasse deux millions de cellules."));
        }
        row.push(
            String::from_utf8(std::mem::take(field))
                .map_err(|_| invalid("Une cellule CSV n’est pas en UTF-8."))?,
        );
        Ok(())
    }
    while at < bytes.len() {
        let b = bytes[at];
        if state == 1 {
            if b == b'"' {
                if bytes.get(at + 1) == Some(&b'"') {
                    field.push(b'"');
                    at += 1;
                } else {
                    state = 2;
                }
            } else {
                field.push(b);
            }
        } else if b == b',' {
            finish(&mut field, &mut row, &mut cells)?;
            state = 0;
            active = true;
        } else if b == b'\r' && bytes.get(at + 1) == Some(&b'\n') {
            finish(&mut field, &mut row, &mut cells)?;
            rows.push(std::mem::take(&mut row));
            state = 0;
            active = false;
            at += 1;
        } else if state == 2 {
            return Err(invalid(
                "Un guillemet CSV fermé doit être suivi d’une virgule ou de CRLF.",
            ));
        } else if b == b'"' {
            if !field.is_empty() {
                return Err(invalid(
                    "Un guillemet CSV ne peut pas commencer au milieu d’un champ.",
                ));
            }
            state = 1;
            active = true;
        } else if b == b'\r' || b == b'\n' {
            return Err(invalid(
                "Les fins de ligne du CSV format 1 doivent être CRLF.",
            ));
        } else {
            field.push(b);
            active = true;
        }
        if field.len() > MAX_FIELD {
            return Err(invalid("Une cellule CSV dépasse 1 Mio."));
        }
        at += 1;
    }
    if state == 1 {
        return Err(invalid("Un champ CSV entre guillemets n’est pas fermé."));
    }
    if active {
        finish(&mut field, &mut row, &mut cells)?;
        rows.push(row);
    }
    Ok(rows)
}

pub(crate) fn context(table: &str, row: usize, column: &str, message: &str) -> AppError {
    AppError::new(
        "invalid_csv",
        format!("Table {table}, ligne {row}, colonne {column} : {message}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn quoted_commas_quotes_unicode_controls_and_embedded_newlines_are_lossless() {
        let fields = vec![
            "Été, hiver".into(),
            "=HYPERLINK(\"x\")".into(),
            "\r\nLigne\n2\t\0".into(),
            String::new(),
            "000123".into(),
        ];
        let mut bytes = Vec::new();
        record(&mut bytes, &fields).unwrap();
        assert_eq!(parse(&bytes).unwrap(), vec![fields]);
        assert!(bytes.ends_with(b"\r\n"));
    }
    #[test]
    fn malformed_csv_is_rejected_without_relaxed_quote_or_encoding_rules() {
        for bytes in [
            b"a\nb\n".as_slice(),
            b"\"a".as_slice(),
            b"\"a\"x\r\n".as_slice(),
            b"a\"b\r\n".as_slice(),
            b"\xef\xbb\xbfa\r\n".as_slice(),
            b"\xff\r\n".as_slice(),
        ] {
            assert!(parse(bytes).is_err(), "{bytes:?}");
        }
        assert_eq!(
            parse(b"a,\"\"").unwrap(),
            vec![vec!["a".to_owned(), String::new()]]
        );
    }
}
