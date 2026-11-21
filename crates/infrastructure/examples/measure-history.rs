//! Repeatable mixed-history measurement using public, transactional APIs.
//! Run with `cargo run -p patrimoine-infrastructure --example measure-history -- 3000`.
use patrimoine_application::{RegistryService, VaultService};
use patrimoine_domain::{Asset, SaveAssetInput, Settings};
use patrimoine_infrastructure::Vault;
use std::time::Instant;

fn revise(vault: &mut Vault, previous: Option<&Asset>, name: &str) -> Asset {
    vault
        .save_asset(SaveAssetInput {
            id: previous.map(|asset| asset.id.clone()),
            expected_revision_id: previous.map(|asset| asset.revision_id.clone()),
            reason: "Mesure de connaissances synthétiques".into(),
            name: name.into(),
            country: "FR".into(),
            category: "other".into(),
            notes: String::new(),
            archived: false,
        })
        .expect("valid synthetic asset")
}

fn main() {
    let mutations: usize = std::env::args()
        .nth(1)
        .map(|value| value.parse().expect("positive mutation count"))
        .unwrap_or(3000);
    assert!((1..=10_000).contains(&mutations));
    let temporary = tempfile::TempDir::new().expect("temporary synthetic vault");
    let vault_path = temporary.path().join("vault");
    let mut vault = Vault::open(&vault_path).expect("open synthetic vault");
    let mut asset = revise(&mut vault, None, "Profil initial");
    let initial = vault.list_registry().expect("snapshot").knowledge_sequence;
    let mut checkpoints = vec![(initial, asset.name.clone())];
    let timer = Instant::now();
    for index in 1..=mutations {
        if index % 100 == 0 {
            asset = revise(&mut vault, Some(&asset), &format!("Profil {index}"));
        } else {
            vault
                .save_settings(Settings {
                    workspace_name: format!("Mesure {index}"),
                    display_currency: "EUR".into(),
                })
                .expect("settings");
        }
        if index == mutations / 2 || index == mutations {
            checkpoints.push((
                vault
                    .list_registry()
                    .expect("checkpoint")
                    .knowledge_sequence,
                asset.name.clone(),
            ));
        }
    }
    let mutations_ms = timer.elapsed().as_secs_f64() * 1000.0;
    let timer = Instant::now();
    let final_asset = revise(&mut vault, Some(&asset), "Profil final");
    let final_write_ms = timer.elapsed().as_secs_f64() * 1000.0;

    drop(vault);
    let timer = Instant::now();
    let mut vault = Vault::open(&vault_path).expect("reopen and validate full history");
    let reopen_ms = timer.elapsed().as_secs_f64() * 1000.0;

    let timer = Instant::now();
    for (knowledge, expected_name) in &checkpoints {
        let historical = vault
            .list_registry_at(Some(*knowledge))
            .expect("historical snapshot");
        assert_eq!(historical.assets[0].name, *expected_name);
        assert_eq!(historical.knowledge_sequence, *knowledge);
    }
    let historical_reads_ms = timer.elapsed().as_secs_f64() * 1000.0;
    let current = vault.list_registry().expect("current snapshot");
    assert_eq!(current.knowledge_sequence, initial + mutations as u64 + 1);
    assert_eq!(current.assets[0].revision_id, final_asset.revision_id);

    let archive = temporary.path().join("history.zip");
    let timer = Instant::now();
    let backup = vault
        .create_backup(&archive)
        .expect("save and validate backup");
    let backup_ms = timer.elapsed().as_secs_f64() * 1000.0;
    let timer = Instant::now();
    let inspection = vault
        .inspect_restore(&archive)
        .expect("inspect saved backup");
    let backup_inspection_ms = timer.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(inspection.archive_sha256, backup.sha256);
    assert_eq!(inspection.mutation_sequence, current.knowledge_sequence);

    println!(
        "{}",
        serde_json::json!({
            "status":"passed", "synthetic_mutations": mutations,
            "registry_revisions": mutations / 100 + 2,
            "settings_mutations": mutations - mutations / 100,
            "initial_knowledge": initial, "final_knowledge": current.knowledge_sequence,
            "mutation_total_ms": mutations_ms, "final_registry_write_ms": final_write_ms,
            "historical_reads_ms": historical_reads_ms, "historical_checkpoints": checkpoints.len(),
            "reopen_with_history_validation_ms": reopen_ms,
            "backup_with_history_validation_ms": backup_ms,
            "backup_inspection_ms": backup_inspection_ms,
            "profile": "mixed settings and registry revisions; timings are observations, not performance thresholds"
        })
    );
}
