//! Run with `cargo run -p patrimoine-infrastructure --example measure-registry`.
use patrimoine_domain::{
    consolidate, Asset, CompanyPolicy, ConsolidationRequest, Entity, ExchangeRate, Fraction,
    Membership, Ownership, Participation, RegistrySnapshot, Valuation,
};
use std::time::Instant;

const START: &str = "2020-01-01";
const OPENING: &str = "2020-01-15";
const END: &str = "2020-12-31";

fn entity(id: String, kind: &str) -> Entity {
    Entity {
        revision_id: format!("{id}-r"),
        display_name: id.clone(),
        id,
        kind: kind.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    }
}

fn rate(date: &str, rate: &str) -> ExchangeRate {
    ExchangeRate {
        id: format!("usd-eur-{date}"),
        revision_id: format!("usd-eur-{date}-r"),
        base_currency: "USD".into(),
        quote_currency: "EUR".into(),
        rate: rate.into(),
        effective_on: date.into(),
        source: Some("Mesure synthétique".into()),
        notes: String::new(),
    }
}

fn snapshot(
    companies: usize,
    assets: usize,
    observations: usize,
    knowledge: u64,
) -> RegistrySnapshot {
    let full = Some(Fraction {
        num: "1".into(),
        den: "1".into(),
    });
    let mut registry = RegistrySnapshot {
        knowledge_sequence: knowledge,
        entities: vec![entity("root".into(), "person")],
        memberships: vec![Membership {
            id: "membership".into(),
            revision_id: "membership-r".into(),
            person_id: "root".into(),
            valid_from: START.into(),
            valid_to: None,
            relationship: String::new(),
        }],
        exchange_rates: vec![rate(START, "1")],
        ..Default::default()
    };
    if observations > 1 {
        registry.exchange_rates.push(rate("2020-07-01", "1.25"));
    }
    for index in 0..companies {
        let id = format!("company-{index}");
        registry.entities.push(entity(id.clone(), "company"));
        registry.participations.push(Participation {
            id: format!("rights-{id}"),
            revision_id: format!("rights-{id}-r"),
            company_id: id.clone(),
            owner_entity_id: if index == 0 {
                "root".into()
            } else {
                format!("company-{}", (index - 1) / 8)
            },
            legal_share: full.clone(),
            economic_share: full.clone(),
            voting_share: None,
            valid_from: START.into(),
            valid_to: None,
        });
        registry.company_policies.push(CompanyPolicy {
            id: format!("policy-{id}"),
            revision_id: format!("policy-{id}-r"),
            company_id: id,
            mode: "detailed".into(),
            coverage_complete: true,
            liability_qualified: true,
            valid_from: START.into(),
            valid_to: None,
        });
    }
    for index in 0..assets {
        let id = format!("asset-{index}");
        registry.assets.push(Asset {
            id: id.clone(),
            revision_id: format!("{id}-r"),
            name: id.clone(),
            country: "FR".into(),
            category: "other".into(),
            notes: String::new(),
            archived: false,
        });
        registry.ownerships.push(Ownership {
            id: format!("own-{id}"),
            revision_id: format!("own-{id}-r"),
            asset_id: id.clone(),
            owner_entity_id: format!("company-{}", index % companies),
            legal_share: full.clone(),
            economic_share: full.clone(),
            valid_from: START.into(),
            valid_to: None,
        });
        for month in 1..=observations {
            registry.valuations.push(Valuation {
                id: format!("valuation-{id}-{month}"),
                revision_id: format!("valuation-{id}-{month}-r"),
                target_kind: "asset".into(),
                target_id: id.clone(),
                amount: if month == 1 { "100" } else { "200" }.into(),
                currency: "USD".into(),
                effective_on: format!("2020-{month:02}-01"),
                source: "Mesure synthétique".into(),
            });
        }
    }
    registry
}

fn measure(label: &str, registry: &RegistrySnapshot, date: &str, per_asset_eur: usize) {
    let request = ConsolidationRequest {
        context: "family".into(),
        person_id: None,
        date: date.into(),
        knowledge_sequence: Some(registry.knowledge_sequence),
    };
    let timer = Instant::now();
    let report = consolidate(registry, &request).expect("valid synthetic snapshot");
    let duration_ms = timer.elapsed().as_secs_f64() * 1000.0;
    assert_eq!(report.contributions.len(), registry.assets.len());
    assert_eq!(
        report.gross_eur,
        format!("{}.00", per_asset_eur * registry.assets.len())
    );
    println!(
        "{}",
        serde_json::json!({
            "status": "passed", "case": label, "date": date, "knowledge": registry.knowledge_sequence,
            "companies": registry.company_policies.len(), "assets": registry.assets.len(),
            "valuations": registry.valuations.len(), "gross_eur": report.gross_eur,
            "consolidation_ms": duration_ms,
        })
    );
}

fn main() {
    measure("wide_graph", &snapshot(384, 384, 1, 2), END, 100);
    let latest = snapshot(32, 400, 12, 2);
    measure("many_valuations", &latest, END, 250);
    measure("historical_date", &latest, OPENING, 100);
    measure("older_knowledge", &snapshot(32, 400, 1, 1), END, 100);
}
