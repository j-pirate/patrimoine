//! The generic history check is exercised on a small typed table so malformed
//! revision roots are not obscured by unrelated financial-domain invariants.
use super::validate_record_history;
use rusqlite::{params, Connection};

const OBJECT: &str = "fe9c7a48-f5dc-411e-9136-c2bb68e8de10";
const FIRST: &str = "be4fc2a2-32e0-4a52-884a-e8cc75378111";
const SECOND: &str = "993c879f-da34-48fa-bcc0-f28d00d0d222";
const THIRD: &str = "4fed6b12-58bb-4c82-a13f-02794bd33333";
const CHANGES: [&str; 3] = [
    "a5c95a2a-8036-4dad-a92f-c286ca200001",
    "a5c95a2a-8036-4dad-a92f-c286ca200002",
    "a5c95a2a-8036-4dad-a92f-c286ca200003",
];

fn chain() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    c.execute_batch(crate::db::SQL).unwrap();
    c.execute_batch(
        "CREATE TABLE samples (
            revision_id TEXT PRIMARY KEY REFERENCES revisions(id),
            note TEXT NOT NULL
        ) STRICT;",
    )
    .unwrap();
    c.execute(
        "INSERT INTO objects VALUES(?1,'document','2026-01-01T00:00:00Z')",
        [OBJECT],
    )
    .unwrap();
    for (index, id) in CHANGES.iter().enumerate() {
        c.execute(
            "INSERT INTO change_sets VALUES(?1,?2,?3,'Source de test',?1)",
            params![id, index + 1, format!("2026-01-0{}T00:00:00Z", index + 1)],
        )
        .unwrap();
    }
    revision(&c, FIRST, CHANGES[0], None, "Source initiale");
    revision(&c, SECOND, CHANGES[1], Some(FIRST), "Source corrigée");
    c
}

fn revision(c: &Connection, id: &str, change: &str, previous: Option<&str>, note: &str) {
    c.execute(
        "INSERT INTO revisions VALUES(?1,?2,?3,(SELECT recorded_at FROM change_sets WHERE id=?3),?4,0,'Révision de test')",
        params![id, OBJECT, change, previous],
    )
    .unwrap();
    c.execute("INSERT INTO samples VALUES(?1,?2)", params![id, note])
        .unwrap();
}

#[test]
fn one_root_and_one_current_revision_form_a_valid_typed_chain() {
    let c = chain();
    validate_record_history(&c, &[("document", "samples")]).unwrap();
    assert!(c
        .prepare("PRAGMA foreign_key_check")
        .unwrap()
        .query([])
        .unwrap()
        .next()
        .unwrap()
        .is_none());
}

#[test]
fn a_second_typed_root_for_the_same_object_is_rejected() {
    let c = chain();
    revision(&c, THIRD, CHANGES[2], None, "Deuxième racine incohérente");
    let error = validate_record_history(&c, &[("document", "samples")]).unwrap_err();
    assert!(error.message.contains("chaîne de révisions"));
}

#[test]
fn a_cycle_with_no_root_is_rejected_even_when_every_revision_has_typed_content() {
    let c = chain();
    c.execute(
        "UPDATE revisions SET supersedes_revision_id=?1 WHERE id=?2",
        params![SECOND, FIRST],
    )
    .unwrap();
    let error = validate_record_history(&c, &[("document", "samples")]).unwrap_err();
    assert!(error.message.contains("chaîne de révisions"));
}
