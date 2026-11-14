//! Capacity regressions use real central directories, including the overhead
//! needed by a CSV archive containing 10,000 distinct documentary originals.
use super::*;
use tempfile::TempDir;

fn archive(path: &Path, names: impl IntoIterator<Item = String>) {
    let mut zip = ZipWriter::new(File::create(path).unwrap());
    for name in names {
        zip.start_file(
            name,
            SimpleFileOptions::default()
                .compression_method(CompressionMethod::Stored)
                .unix_permissions(0o600),
        )
        .unwrap();
    }
    zip.finish().unwrap();
}

fn numbered(count: usize) -> impl Iterator<Item = String> {
    (0..count).map(|index| format!("tables/capacity-{index:05}.csv"))
}

#[test]
fn ten_thousand_originals_leave_room_for_all_csv_tables_and_metadata() {
    let temporary = TempDir::new().unwrap();
    let path = temporary.path().join("complete-catalogue.zip");
    let documents = (0..10_000).map(|index| format!("documents/sha256/00/{index:064x}"));
    let tables = (0..62).map(|index| format!("tables/table-{index:02}.csv"));
    let metadata = [
        "manifest.json".to_owned(),
        "dictionary/schema.json".to_owned(),
        "recovery/LISEZMOI.md".to_owned(),
    ];
    archive(&path, documents.chain(tables).chain(metadata));

    let zip = ZipArchive::new(File::open(&path).unwrap()).unwrap();
    assert_eq!(zip.len(), 10_065);
    drop(zip);
    validate_zip_directory_with(&path, |_| true).unwrap();
}

#[test]
fn entry_budget_accepts_its_boundary_and_rejects_the_next_real_entry_without_mutation() {
    let temporary = TempDir::new().unwrap();
    let accepted = temporary.path().join("at-budget.zip");
    archive(&accepted, numbered(11_000));
    validate_zip_directory_with(&accepted, |_| true).unwrap();

    let refused = temporary.path().join("over-budget.zip");
    archive(&refused, numbered(11_001));
    let before = fs::read(&refused).unwrap();
    assert!(validate_zip_directory_with(&refused, |_| true).is_err());
    assert_eq!(fs::read(&refused).unwrap(), before);
    assert_eq!(
        ZipArchive::new(File::open(&refused).unwrap())
            .unwrap()
            .len(),
        11_001,
        "refusal must not truncate the source archive"
    );
}

#[test]
fn larger_database_limit_is_exclusive_to_the_canonical_sqlite_entry() {
    assert_eq!(entry_limit("data/patrimoine.sqlite3"), 256 * 1024 * 1024);
    for name in [
        "tables/treasury_transactions.csv",
        "documents/sha256/00/0000000000000000000000000000000000000000000000000000000000000000",
        "data/other.sqlite3",
        "data/PATRIMOINE.sqlite3",
        "other/data/patrimoine.sqlite3",
    ] {
        assert_eq!(entry_limit(name), 128 * 1024 * 1024, "{name}");
    }
    assert_eq!(MAX_TOTAL, 512 * 1024 * 1024);
}
