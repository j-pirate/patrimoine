//! Lossless CSV cells and schema-derived nullable markers; no numeric coercion.
use crate::{db, files::*};
use patrimoine_domain::{AppError, ErrorCode, Result};
use rusqlite::{types::ValueRef, Connection};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};

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

/// Reads one logical record at a time, including newlines inside quoted cells.
/// Limits apply to decoded cell bytes, all cells (including the header), and
/// source bytes. A schema column limit also bounds malformed, very wide rows.
pub(crate) struct Reader<R> {
    source: R,
    bytes: u64,
    cells: usize,
    records: usize,
    prefix: [u8; 3],
    max_columns: Option<usize>,
    limits: Limits,
}

struct Limits {
    field: usize,
    cells: usize,
    bytes: u64,
}

impl<R: BufRead> Reader<R> {
    pub(crate) fn new(source: R, max_columns: Option<usize>) -> Self {
        Self {
            source,
            bytes: 0,
            cells: 0,
            records: 0,
            prefix: [0; 3],
            max_columns,
            limits: Limits {
                field: MAX_FIELD,
                cells: MAX_CELLS,
                bytes: crate::archives::MAX_FILE,
            },
        }
    }

    pub(crate) fn line(&self) -> usize {
        self.records + 1
    }

    fn finish(
        field: &mut Vec<u8>,
        row: &mut Vec<String>,
        cells: &mut usize,
        limits: &Limits,
        max_columns: Option<usize>,
    ) -> Result<()> {
        *cells += 1;
        if *cells > limits.cells {
            return Err(invalid("Le CSV dépasse deux millions de cellules."));
        }
        if max_columns.is_some_and(|columns| row.len() >= columns) {
            return Err(invalid("Le nombre de cellules diffère de l’en-tête."));
        }
        row.push(
            String::from_utf8(std::mem::take(field))
                .map_err(|_| invalid("Le CSV doit être en UTF-8."))?,
        );
        Ok(())
    }

    pub(crate) fn next_row(&mut self) -> Result<Option<Vec<String>>> {
        let mut row = Vec::new();
        let mut field = Vec::new();
        // 0 = unquoted cell, 1 = quoted cell, 2 = closing quote.
        let mut state = 0u8;
        let mut cr = false;
        let mut active = false;
        loop {
            let input = self.source.fill_buf().map_err(io)?;
            if input.is_empty() {
                break;
            }
            let mut consumed = 0;
            let mut complete = false;
            while consumed < input.len() {
                let b = input[consumed];
                consumed += 1;
                self.bytes += 1;
                if self.bytes > self.limits.bytes {
                    return Err(invalid("Une table CSV dépasse 128 Mio."));
                }
                if self.bytes <= 3 {
                    self.prefix[self.bytes as usize - 1] = b;
                    if self.bytes == 3 && self.prefix == [0xef, 0xbb, 0xbf] {
                        return Err(invalid("Le CSV format 1 ne doit pas contenir de BOM."));
                    }
                }
                if cr {
                    if b != b'\n' {
                        return Err(if state == 2 {
                            invalid(
                                "Un guillemet CSV fermé doit être suivi d’une virgule ou de CRLF.",
                            )
                        } else {
                            invalid("Les fins de ligne du CSV format 1 doivent être CRLF.")
                        });
                    }
                    Self::finish(
                        &mut field,
                        &mut row,
                        &mut self.cells,
                        &self.limits,
                        self.max_columns,
                    )?;
                    complete = true;
                    break;
                }
                if state == 1 {
                    if b == b'"' {
                        state = 2;
                    } else {
                        field.push(b);
                    }
                } else if state == 2 && b == b'"' {
                    // A doubled quote belongs to the same quoted cell, even
                    // when its two bytes straddle separate input buffers.
                    field.push(b'"');
                    state = 1;
                } else if b == b',' {
                    Self::finish(
                        &mut field,
                        &mut row,
                        &mut self.cells,
                        &self.limits,
                        self.max_columns,
                    )?;
                    state = 0;
                    active = true;
                } else if b == b'\r' {
                    cr = true;
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
                } else if b == b'\n' {
                    return Err(invalid(
                        "Les fins de ligne du CSV format 1 doivent être CRLF.",
                    ));
                } else {
                    field.push(b);
                    active = true;
                }
                if field.len() > self.limits.field {
                    return Err(invalid("Une cellule CSV dépasse 1 Mio."));
                }
            }
            self.source.consume(consumed);
            if complete {
                self.records += 1;
                return Ok(Some(row));
            }
        }
        if cr {
            return Err(if state == 2 {
                invalid("Un guillemet CSV fermé doit être suivi d’une virgule ou de CRLF.")
            } else {
                invalid("Les fins de ligne du CSV format 1 doivent être CRLF.")
            });
        }
        if state == 1 {
            return Err(invalid("Un champ CSV entre guillemets n’est pas fermé."));
        }
        if active {
            Self::finish(
                &mut field,
                &mut row,
                &mut self.cells,
                &self.limits,
                self.max_columns,
            )?;
            self.records += 1;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<Vec<String>>> {
    let mut reader = Reader::new(bytes, None);
    let mut rows = Vec::new();
    while let Some(row) = reader.next_row()? {
        rows.push(row);
    }
    Ok(rows)
}

pub(crate) fn context(table: &str, row: usize, column: &str, message: &str) -> AppError {
    AppError::new(
        ErrorCode::InvalidCsv,
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

    #[test]
    fn streaming_records_preserve_quotes_crlf_and_utf8_across_tiny_buffers() {
        use std::io::BufReader;

        let fields = vec![
            "été,東京".into(),
            "\"\r\n\"\n\0".into(),
            String::new(),
            "000123".into(),
        ];
        let mut bytes = Vec::new();
        record(&mut bytes, &fields).unwrap();
        bytes.extend_from_slice(b"plain,\"\",last,");
        let last = vec!["plain".into(), String::new(), "last".into(), String::new()];
        for capacity in 1..=17 {
            let mut reader = Reader::new(
                BufReader::with_capacity(capacity, bytes.as_slice()),
                Some(4),
            );
            assert_eq!(reader.line(), 1);
            assert_eq!(
                reader.next_row().unwrap(),
                Some(fields.clone()),
                "{capacity}"
            );
            assert_eq!(reader.line(), 2);
            assert_eq!(reader.next_row().unwrap(), Some(last.clone()), "{capacity}");
            assert_eq!(reader.next_row().unwrap(), None);
        }
    }

    #[test]
    fn streaming_returns_a_record_before_a_later_malformed_record() {
        let mut reader = Reader::new(b"first\r\n\"unclosed\r\nnext".as_slice(), Some(1));
        assert_eq!(reader.next_row().unwrap(), Some(vec!["first".into()]));
        assert_eq!(reader.line(), 2);
        assert!(reader.next_row().unwrap_err().message.contains("pas fermé"));
    }

    #[test]
    fn a_one_mib_multiline_cell_is_lossless_and_one_extra_byte_is_rejected() {
        let exact = format!("{}\r\n\"x", "é".repeat((MAX_FIELD - 4) / 2));
        assert_eq!(exact.len(), MAX_FIELD);
        let mut bytes = Vec::new();
        record(&mut bytes, std::slice::from_ref(&exact)).unwrap();
        let mut reader = Reader::new(
            std::io::BufReader::with_capacity(4093, bytes.as_slice()),
            Some(1),
        );
        assert_eq!(reader.next_row().unwrap(), Some(vec![exact.clone()]));
        assert_eq!(reader.next_row().unwrap(), None);

        bytes.clear();
        record(&mut bytes, &[format!("{exact}x")]).unwrap();
        let mut reader = Reader::new(bytes.as_slice(), Some(1));
        assert!(reader.next_row().unwrap_err().message.contains("1 Mio"));
    }

    #[test]
    fn streaming_limits_count_source_bytes_and_cells_including_the_header() {
        let mut bytes = b"a,b\r\n".to_vec();
        record(&mut bytes, &["\"\r\n\"".into(), "x".into()]).unwrap();
        bytes.extend_from_slice(b"c,d\r\n");
        let mut reader = Reader::new(bytes.as_slice(), Some(2));
        reader.limits.cells = 4;
        assert_eq!(
            reader.next_row().unwrap(),
            Some(vec!["a".into(), "b".into()])
        );
        assert_eq!(
            reader.next_row().unwrap(),
            Some(vec!["\"\r\n\"".into(), "x".into()])
        );
        assert!(reader
            .next_row()
            .unwrap_err()
            .message
            .contains("millions de cellules"));

        let mut reader = Reader::new(bytes.as_slice(), Some(2));
        reader.limits.bytes = bytes.len() as u64;
        while reader.next_row().unwrap().is_some() {}
        let mut reader = Reader::new(bytes.as_slice(), Some(2));
        reader.limits.bytes = bytes.len() as u64 - 1;
        reader.next_row().unwrap();
        reader.next_row().unwrap();
        assert!(reader.next_row().unwrap_err().message.contains("128 Mio"));

        let mut reader = Reader::new(b"a,b,c,never retained\r\n".as_slice(), Some(2));
        assert!(reader
            .next_row()
            .unwrap_err()
            .message
            .contains("nombre de cellules"));
        assert_eq!(reader.cells, 3);
    }

    #[test]
    fn streaming_rejects_encoding_and_line_errors_even_when_split_between_buffers() {
        for bytes in [
            b"\xef\xbb\xbfa\r\n".as_slice(),
            b"\"\xc3\"\r\n".as_slice(),
            b"\"a\"\r".as_slice(),
            b"a\rb\r\n".as_slice(),
            b"\"a\"x\r\n".as_slice(),
        ] {
            for capacity in 1..=4 {
                let mut reader =
                    Reader::new(std::io::BufReader::with_capacity(capacity, bytes), None);
                assert!(reader.next_row().is_err(), "{bytes:?}, {capacity}");
            }
        }
    }
}
