//! Run with `cargo run -p patrimoine-infrastructure --example measure-cash-capacity`.
use patrimoine_domain::{
    validate_operation_cash_capacity, CashLink, CompanySnapshot, MetalSnapshot, TreasuryEntry,
    TreasurySnapshot, TreasuryTransaction,
};
use std::time::Instant;

fn main() {
    let mut treasury = TreasurySnapshot::default();
    let metals = MetalSnapshot::default();
    let companies = CompanySnapshot::default();
    let line_count = 4_000;
    for index in 0..line_count {
        let revision = format!("transaction-{index}-revision");
        treasury.transactions.push(TreasuryTransaction {
            id: format!("transaction-{index}"),
            revision_id: revision.clone(),
            kind: "receipt".into(),
            effective_on: "2026-01-01".into(),
            label: String::new(),
            reference: String::new(),
            notes: String::new(),
            entries: vec![TreasuryEntry {
                line_id: "principal".into(),
                account_id: "bank".into(),
                amount: "100".into(),
                role: "principal".into(),
            }],
        });
        for (source, amount) in [("first", "40"), ("second", "60")] {
            let id = format!("{source}-{index}");
            treasury.cash_links.push(CashLink {
                revision_id: format!("{id}-revision"),
                id: id.clone(),
                source_kind: "rent_payment".into(),
                source_revision_id: format!("{id}-source-revision"),
                source_id: id,
                transaction_revision_id: revision.clone(),
                line_id: "principal".into(),
                amount: amount.into(),
                component: "principal".into(),
                side: "credit".into(),
                notes: String::new(),
            });
        }
    }
    let timer = Instant::now();
    validate_operation_cash_capacity(&treasury, &metals, &companies).expect("exact capacity");
    let duration_ms = timer.elapsed().as_secs_f64() * 1000.0;
    treasury.cash_links.last_mut().unwrap().amount = "60.01".into();
    assert!(validate_operation_cash_capacity(&treasury, &metals, &companies).is_err());
    println!(
        "{}",
        serde_json::json!({
            "status": "passed", "lines": line_count, "cash_links": treasury.cash_links.len(),
            "validation_ms": duration_ms, "over_capacity_rejected": true,
        })
    );
}
