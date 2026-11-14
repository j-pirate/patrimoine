//! Release-only qualification driver. Fixture seeding is deliberately separate
//! from measurements; opening always runs the application's real validation.
use patrimoine_application::{
    DashboardService, DocumentService, FinancialService, PortabilityService, RegistryService,
    TreasuryService, VaultService,
};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use serde_json::{json, Value};
use std::{
    fs,
    io::{self, BufRead, Write},
    path::Path,
    time::Instant,
};

fn require(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(AppError::new("benchmark_oracle", message))
    }
}

fn io(error: impl std::fmt::Display) -> AppError {
    AppError::new("benchmark_io", error.to_string())
}

fn run(root: &Path, operation: &str) -> Result<Value> {
    let started = Instant::now();
    let mut vault = Vault::open(root)?;
    let open_seconds = started.elapsed().as_secs_f64();
    if operation == "init" {
        return Ok(json!({"operation":operation,"open_seconds":open_seconds,"status":"passed"}));
    }
    let expected: Value =
        serde_json::from_slice(&fs::read(root.join("expected.json")).map_err(io)?).map_err(io)?;
    let mut output = run_operation(root, &mut vault, &expected, operation)?;
    output["open_seconds"] = json!(open_seconds);
    output["total_seconds"] = json!(started.elapsed().as_secs_f64());
    Ok(output)
}

fn run_operation(
    root: &Path,
    vault: &mut Vault,
    expected: &Value,
    operation: &str,
) -> Result<Value> {
    let active = vault.status()?;
    require(active.mutation_sequence == 2, "La fixture doit être à K=2.")?;
    let begin = Instant::now();
    let q = FinancialReportRequest {
        context: "family".into(),
        person_id: None,
        date: "2025-12-31".into(),
        knowledge_sequence: Some(2),
    };
    let result = match operation {
        "open" => {
            json!({"knowledge_sequence":active.mutation_sequence,"documents":active.document_count})
        }
        "registry" => {
            let r = vault.list_registry_at(Some(2))?;
            require(
                r.assets.len() == expected["assets"].as_u64().unwrap() as usize,
                "Nombre d’actifs incorrect.",
            )?;
            require(
                r.entities.len()
                    == (expected["people"].as_u64().unwrap()
                        + expected["companies"].as_u64().unwrap()) as usize,
                "Nombre d’entités incorrect.",
            )?;
            json!({"assets":r.assets.len(),"entities":r.entities.len(),"valuations":r.valuations.len()})
        }
        "treasury" => {
            let r = vault.list_treasury_at(Some(2))?;
            require(
                r.transactions.len() == expected["movements"].as_u64().unwrap() as usize,
                "Nombre de mouvements incorrect.",
            )?;
            json!({"transactions":r.transactions.len(),"accounts":r.accounts.len()})
        }
        "documents" => {
            let r = vault.list_document_library_at(Some(2))?;
            require(
                r.documents.len() == expected["documents"].as_u64().unwrap() as usize,
                "Nombre de documents incorrect.",
            )?;
            require(
                r.links.len() == expected["documents"].as_u64().unwrap() as usize,
                "Liens documentaires incomplets.",
            )?;
            json!({"documents":r.documents.len(),"links":r.links.len(),"targets":r.targets.len()})
        }
        "account" => {
            let r = vault.get_treasury_report(TreasuryReportRequest {
                account_id: expected["account_id"].as_str().unwrap().into(),
                date: q.date.clone(),
                knowledge_sequence: Some(2),
            })?;
            require(
                r.balance_original.as_deref() == expected["bank_balance"].as_str(),
                "Le solde exact du journal diffère de l’oracle indépendant.",
            )?;
            json!({"balance_original":r.balance_original,"currency":r.currency})
        }
        "financial" | "historical" | "individual" => {
            let mut query = q;
            if operation == "historical" {
                query.knowledge_sequence = Some(1);
            }
            if operation == "individual" {
                query.context = "person".into();
                query.person_id = Some(expected["owner_id"].as_str().unwrap().into());
            }
            let r = vault.get_financial_report(query)?;
            let oracle = if operation == "historical" {
                "historical_assets_eur"
            } else {
                "current_assets_eur"
            };
            require(
                r.assets_eur.as_deref() == expected[oracle].as_str()
                    && r.net_eur == r.assets_eur
                    && r.liabilities_eur.as_deref() == Some("0.00"),
                "Le bilan exact diffère de l’oracle indépendant.",
            )?;
            json!({"assets_eur":r.assets_eur,"net_eur":r.net_eur,"positions":r.positions.len(),"warnings":r.warnings.len(),"knowledge_sequence":r.knowledge_sequence})
        }
        "fx" => {
            let r = vault.convert_currency(FxQuery {
                amount: "123.45".into(),
                from_currency: "TND".into(),
                to_currency: "EUR".into(),
                date: q.date,
                knowledge_sequence: Some(2),
            })?;
            require(
                r.amount_converted.as_deref() == Some("37.04"),
                "La conversion exacte diffère de l’oracle.",
            )?;
            json!({"amount_converted":r.amount_converted,"status":r.status})
        }
        "dashboard" => {
            let r = vault.get_dashboard(DashboardRequest {
                context: "family".into(),
                entity_id: None,
                date: q.date,
                from_date: "2025-12-01".into(),
                interval: "monthly".into(),
                knowledge_sequence: Some(2),
                filters: DashboardFilters::default(),
            })?;
            let totals = r
                .totals
                .as_ref()
                .ok_or_else(|| AppError::new("benchmark_oracle", "Totaux absents."))?;
            require(
                totals.assets_eur.as_deref() == expected["current_assets_eur"].as_str()
                    && r.history.len() == 2,
                "La synthèse ou son calendrier diffère de l’oracle.",
            )?;
            json!({"assets_eur":totals.assets_eur,"members":r.members.len(),"historical_points":r.history.len(),"positions":r.positions.len()})
        }
        "backup" => {
            let path = root.parent().unwrap().join("benchmark-backup.zip");
            let r = vault.create_backup(&path)?;
            require(
                r.document_count == expected["documents"].as_u64().unwrap()
                    && r.blob_count == expected["unique_blobs"].as_u64().unwrap(),
                "Sauvegarde documentaire incomplète.",
            )?;
            json!({"bytes":fs::metadata(path).map_err(io)?.len(),"documents":r.document_count,"blobs":r.blob_count})
        }
        "csv_export" => {
            let path = root.parent().unwrap().join("benchmark-csv.zip");
            let r = vault.export_csv_archive(&path)?;
            require(
                r.row_count == expected["rows"].as_u64().unwrap()
                    && r.table_count == expected["table_counts"].as_object().unwrap().len() as u64
                    && r.document_count == expected["documents"].as_u64().unwrap()
                    && r.blob_count == expected["unique_blobs"].as_u64().unwrap(),
                "Export CSV incomplet.",
            )?;
            json!({"bytes":fs::metadata(path).map_err(io)?.len(),"tables":r.table_count,"rows":r.row_count,"documents":r.document_count,"blobs":r.blob_count})
        }
        "csv_inspect" => {
            let path = root.parent().unwrap().join("benchmark-csv.zip");
            let r = vault.inspect_csv_import(&path)?;
            require(
                r.document_count == expected["documents"].as_u64().unwrap(),
                "Documents incomplets dans la candidate CSV.",
            )?;
            require(
                r.blob_count == expected["unique_blobs"].as_u64().unwrap(),
                "Blobs incomplets dans la candidate CSV.",
            )?;
            json!({"documents":r.document_count,"blobs":r.blob_count,"unchanged_target_sequence":r.mutation_sequence})
        }
        "backup_roundtrip" | "csv_roundtrip" => {
            let csv = operation == "csv_roundtrip";
            let name = if csv { "csv" } else { "backup" };
            let path = root.parent().unwrap().join(format!("benchmark-{name}.zip"));
            let target_root = root
                .parent()
                .unwrap()
                .join(format!("benchmark-{name}-restored"));
            require(
                !target_root.exists(),
                "Le coffre cible de reconstruction existe déjà.",
            )?;
            let mut target = Vault::open(&target_root)?;
            let before = target.status()?;
            let inspection = if csv {
                target.inspect_csv_import(&path)?
            } else {
                target.inspect_restore(&path)?
            };
            let inspected = target.status()?;
            require(
                inspected.generation_id == before.generation_id
                    && inspected.mutation_sequence == before.mutation_sequence,
                "L’inspection a modifié la cible.",
            )?;
            require(
                inspection.document_count == expected["documents"].as_u64().unwrap()
                    && inspection.blob_count == expected["unique_blobs"].as_u64().unwrap(),
                "La reconstruction a perdu des documents.",
            )?;
            let restored = target.activate_restore(&inspection.token, true, None)?;
            require(
                restored.mutation_sequence == if csv { 3 } else { 2 }
                    && restored.generation_id != before.generation_id,
                "La reconstruction a un K incorrect.",
            )?;
            json!({"target":target_root,"knowledge_sequence":restored.mutation_sequence,"documents":restored.document_count,"inspection_target_unchanged":true,"exact_database_and_document_comparison":"separate Python typed-row SHA-256 proof"})
        }
        _ => {
            return Err(AppError::new(
                "benchmark_argument",
                "Opération de qualification inconnue.",
            ))
        }
    };
    let operation_seconds = begin.elapsed().as_secs_f64();
    let after = vault.status()?;
    require(
        after.mutation_sequence == 2 && after.generation_id == active.generation_id,
        "La mesure a modifié le coffre source.",
    )?;
    Ok(
        json!({"status":"passed","operation":operation,"operation_seconds":operation_seconds,"result":result,"source_unchanged":true}),
    )
}

fn output(result: &Result<Value>, operation: &str, cpu_before: Option<(u64, u64)>) -> Value {
    let mut output = match &result {
        Ok(value) => value.clone(),
        Err(error) => {
            json!({"status":"error","operation":operation,"error":{"code":error.code,"message":error.message}})
        }
    };
    if let Ok(status) = fs::read_to_string("/proc/self/status") {
        for name in ["VmHWM", "VmRSS"] {
            if let Some(line) = status
                .lines()
                .find(|line| line.starts_with(&format!("{name}:")))
            {
                output[format!("{name}_kib")] = json!(line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or("0")
                    .parse::<u64>()
                    .unwrap_or(0));
            }
        }
    }
    if let Some((user, system)) = cpu_ticks() {
        output["user_cpu_ticks"] = json!(user);
        output["system_cpu_ticks"] = json!(system);
        if let Some((before_user, before_system)) = cpu_before {
            output["operation_user_cpu_ticks"] = json!(user - before_user);
            output["operation_system_cpu_ticks"] = json!(system - before_system);
        }
    }
    output
}

fn cpu_ticks() -> Option<(u64, u64)> {
    if let Ok(stat) = fs::read_to_string("/proc/self/stat") {
        if let Some(end) = stat.rfind(')') {
            let fields: Vec<_> = stat[end + 1..].split_whitespace().collect();
            return Some((fields.get(11)?.parse().ok()?, fields.get(12)?.parse().ok()?));
        }
    }
    None
}

fn emit(output: &Value) {
    println!("{}", serde_json::to_string(&output).unwrap());
    io::stdout().flush().unwrap();
}

fn session(root: &Path) -> Result<()> {
    let started = Instant::now();
    let mut vault = Vault::open(root)?;
    let expected: Value =
        serde_json::from_slice(&fs::read(root.join("expected.json")).map_err(io)?).map_err(io)?;
    let mut opened = run_operation(root, &mut vault, &expected, "open")?;
    opened["open_seconds"] = json!(started.elapsed().as_secs_f64());
    opened["total_seconds"] = json!(started.elapsed().as_secs_f64());
    emit(&output(&Ok(opened), "open", None));
    for line in io::stdin().lock().lines() {
        let operation = line.map_err(io)?;
        if operation == "quit" {
            break;
        }
        let reset = fs::write("/proc/self/clear_refs", "5").is_ok();
        let cpu_before = cpu_ticks();
        let result = run_operation(root, &mut vault, &expected, &operation);
        let mut measured = output(&result, &operation, cpu_before);
        measured["memory_peak_reset"] = json!(reset);
        measured["shared_validated_session"] = json!(true);
        emit(&measured);
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: qualify_performance COFFRE OPERATION|--session");
        std::process::exit(2);
    }
    if args[2] == "--session" {
        if let Err(error) = session(Path::new(&args[1])) {
            emit(&output(&Err(error), "session", None));
            std::process::exit(1);
        }
        return;
    }
    let result = run(Path::new(&args[1]), &args[2]);
    emit(&output(&result, &args[2], None));
    if result.is_err() {
        std::process::exit(1);
    }
}
