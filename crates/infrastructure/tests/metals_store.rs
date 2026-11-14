//! Independent inventory, revision and atomic cash oracles on real SQLite vaults.
use patrimoine_application::{MetalService, RegistryService, TreasuryService, VaultService};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use serde_json::{json, Value};
use tempfile::TempDir;

fn fraction(num: &str, den: &str) -> Option<Fraction> {
    Some(Fraction {
        num: num.into(),
        den: den.into(),
    })
}
fn person(v: &mut Vault, name: &str) -> Entity {
    v.save_entity(SaveEntityInput {
        id: None,
        expected_revision_id: None,
        reason: "Test synthétique".into(),
        kind: "person".into(),
        display_name: name.into(),
        first_name: None,
        last_name: None,
        birth_date: None,
        residence_country: Some("FR".into()),
        preferred_currency: "EUR".into(),
        activity: String::new(),
        notes: String::new(),
        archived: false,
    })
    .unwrap()
}
fn asset(v: &mut Vault, owner: &Entity, name: &str) -> Asset {
    let a = v
        .save_asset(SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: "Test synthétique".into(),
            name: name.into(),
            country: "FR".into(),
            category: "autre".into(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    v.save_ownership(SaveOwnershipInput {
        id: None,
        expected_revision_id: None,
        reason: "Titre synthétique".into(),
        asset_id: a.id.clone(),
        owner_entity_id: owner.id.clone(),
        legal_share: fraction("1", "1"),
        economic_share: fraction("1", "1"),
        valid_from: "2020-01-01".into(),
        valid_to: None,
    })
    .unwrap();
    a
}
fn lot_input(asset: &Asset, owner: &Entity) -> SaveMetalLotInput {
    SaveMetalLotInput {
        id: None,
        expected_revision_id: None,
        reason: "Lot synthétique".into(),
        asset_id: asset.id.clone(),
        metal: "gold".into(),
        form: "coin".into(),
        designation: "Pièces synthétiques".into(),
        unit_weight: "31.1034768".into(),
        weight_unit: "g".into(),
        purity: "0.9".into(),
        tracking_start: "2025-01-01".into(),
        cost_currency: "EUR".into(),
        legal_owner_entity_id: owner.id.clone(),
        country: "FR".into(),
        storage_location: "Coffre A".into(),
        valuation_basis: "intrinsic".into(),
        notes: String::new(),
        archived: false,
    }
}
fn movement(
    lot: &MetalLot,
    kind: &str,
    date: &str,
    quantity: &str,
    cost: Option<&str>,
) -> SaveMetalMovementInput {
    SaveMetalMovementInput {
        id: None,
        expected_revision_id: None,
        reason: "Mouvement synthétique".into(),
        lot_id: lot.id.clone(),
        kind: kind.into(),
        effective_on: date.into(),
        quantity_change: quantity.into(),
        cost_amount: cost.map(str::to_owned),
        trade_amount: None,
        trade_fees: "0".into(),
        trade_currency: None,
        cash_entity_id: String::new(),
        counterparty_entity_id: None,
        country: "FR".into(),
        storage_location: "Coffre A".into(),
        unlinked_reason: String::new(),
        notes: String::new(),
    }
}
fn quote(v: &mut Vault) -> MetalQuote {
    v.save_metal_quote(SaveMetalQuoteInput {
        id: None,
        expected_revision_id: None,
        reason: "Cours synthétique".into(),
        metal: "gold".into(),
        effective_on: "2025-01-01".into(),
        price: "60".into(),
        currency: "EUR".into(),
        weight_unit: "g".into(),
        source: "Oracle indépendant".into(),
        notes: String::new(),
    })
    .unwrap()
}
fn transaction(
    account: &TreasuryAccount,
    kind: &str,
    date: &str,
    amount: &str,
) -> SaveTreasuryTransactionInput {
    SaveTreasuryTransactionInput {
        id: None,
        expected_revision_id: None,
        reason: "Banque synthétique".into(),
        kind: kind.into(),
        effective_on: date.into(),
        label: "Journal métal synthétique".into(),
        reference: String::new(),
        notes: String::new(),
        entries: vec![TreasuryEntry {
            line_id: "cash-line".into(),
            account_id: account.id.clone(),
            amount: amount.into(),
            role: if kind == "opening" {
                "opening"
            } else {
                "principal"
            }
            .into(),
        }],
    }
}
fn fixture() -> (TempDir, Vault, Entity, MetalLot, TreasuryAccount) {
    let temp = TempDir::new().unwrap();
    let mut v = Vault::open(temp.path().join("vault")).unwrap();
    let owner = person(&mut v, "Alice synthétique");
    let a = asset(&mut v, &owner, "Lot physique");
    let lot = v.save_metal_lot(lot_input(&a, &owner)).unwrap();
    let bank_asset = asset(&mut v, &owner, "Banque");
    let bank = v
        .save_treasury_account(SaveTreasuryAccountInput {
            id: None,
            expected_revision_id: None,
            reason: "Banque synthétique".into(),
            asset_id: bank_asset.id,
            kind: "bank".into(),
            currency: "EUR".into(),
            tracking_start: "2025-01-01".into(),
            legal_owner_entity_id: owner.id.clone(),
            bank_name: "Banque test".into(),
            reference: "TEST".into(),
            country: "FR".into(),
            storage_location: String::new(),
            notes: String::new(),
            archived: false,
        })
        .unwrap();
    v.save_treasury_transaction(transaction(&bank, "opening", "2025-01-01", "1000"))
        .unwrap();
    (temp, v, owner, lot, bank)
}
fn report(v: &Vault, lot: &MetalLot, k: Option<u64>) -> MetalReport {
    v.get_metal_report(MetalReportRequest {
        lot_id: lot.id.clone(),
        date: "2025-12-31".into(),
        knowledge_sequence: k,
    })
    .unwrap()
}
fn bank_balance(v: &Vault, bank: &TreasuryAccount) -> String {
    v.get_treasury_report(TreasuryReportRequest {
        account_id: bank.id.clone(),
        date: "2025-12-31".into(),
        knowledge_sequence: None,
    })
    .unwrap()
    .balance_original
    .unwrap()
}
fn state(v: &Vault) -> Value {
    json!({"registry":v.list_registry().unwrap(),"metal":v.list_metals_at(None).unwrap(),"cash":v.list_treasury_at(None).unwrap(),"history":v.list_registry_history().unwrap()})
}
fn sale_input(
    v: &Vault,
    owner: &Entity,
    lot: &MetalLot,
    bank: &TreasuryAccount,
) -> PostMetalMovementInput {
    let mut m = movement(lot, "sale", "2025-02-01", "-2", None);
    m.trade_amount = Some("260".into());
    m.trade_fees = "10".into();
    m.trade_currency = Some("EUR".into());
    m.cash_entity_id = owner.id.clone();
    PostMetalMovementInput {
        expected_sequence: v.list_metals_at(None).unwrap().knowledge_sequence,
        movement: m,
        transaction: Some(transaction(bank, "asset_trade", "2025-02-01", "250")),
        existing_transaction_revision_id: None,
        links: vec![PostedMetalCashLink {
            line_id: "cash-line".into(),
            amount: "250".into(),
        }],
    }
}

#[test]
fn explicit_opening_and_exact_fine_weight_survive_persistence_without_a_second_valuation() {
    let (_temp, mut v, _owner, lot, _bank) = fixture();
    quote(&mut v);
    assert!(report(&v, &lot, None).quantity_remaining.is_none());
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "2", Some("2000")))
        .unwrap();
    let r = report(&v, &lot, None);
    assert_eq!(r.exact_quantity, fraction("2", "1"));
    assert_eq!(r.exact_fine_weight_g, fraction("174957057", "3125000"));
    assert_eq!(r.intrinsic_eur.as_deref(), Some("3359.18"));
    assert_eq!(r.cost_remaining_original.as_deref(), Some("2000.00"));
    assert!(v.list_registry().unwrap().valuations.is_empty());
    v.save_valuation(SaveValuationInput {
        id: None,
        expected_revision_id: None,
        reason: "Ancienne estimation générique".into(),
        target_kind: "asset".into(),
        target_id: lot.asset_id.clone(),
        amount: "999999".into(),
        currency: "EUR".into(),
        effective_on: "2025-01-01".into(),
        source: "Ne doit pas remplacer le stock métal dans le pont".into(),
    })
    .unwrap();
    let bridge = v
        .explain_variation(VariationRequest {
            context: "family".into(),
            person_id: None,
            from_date: "2025-01-01".into(),
            to_date: "2025-02-01".into(),
            knowledge_sequence: None,
        })
        .unwrap();
    assert!(bridge
        .warnings
        .iter()
        .any(|w| w.code == "metal_stocks_excluded_from_bridge"));
    assert!(!bridge
        .opening
        .contributions
        .iter()
        .any(|c| c.object_id == lot.asset_id));
    let unchanged = state(&v);
    assert!(v
        .save_metal_movement(movement(&lot, "opening", "2025-01-01", "2", Some("2000")))
        .is_err());
    assert_eq!(state(&v), unchanged);
}

#[test]
fn partial_sale_allocates_original_cost_and_posts_its_cash_once() {
    let (_temp, mut v, owner, lot, bank) = fixture();
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "10", Some("1000")))
        .unwrap();
    quote(&mut v);
    v.post_metal_movement(sale_input(&v, &owner, &lot, &bank))
        .unwrap();
    let r = report(&v, &lot, None);
    assert_eq!(r.exact_quantity, fraction("8", "1"));
    assert_eq!(r.cost_remaining_original.as_deref(), Some("800.00"));
    assert_eq!(r.realized_gain_eur.as_deref(), Some("50.00"));
    assert_eq!(
        r.cost_allocations[0].exact_cost_original,
        fraction("200", "1")
    );
    assert_eq!(bank_balance(&v, &bank), "1250.00");
    let s = v.list_metals_at(None).unwrap();
    assert_eq!(s.cash_links.len(), 1);
    assert_eq!(v.list_treasury_at(None).unwrap().transactions.len(), 2);
    assert!(v
        .list_registry_history()
        .unwrap()
        .events
        .iter()
        .any(|e| e.kind == "metal_cash_link"));
}

#[test]
fn correcting_an_atomic_trade_reuses_its_operation_and_preserves_old_knowledge() {
    let (_temp, mut v, owner, lot, bank) = fixture();
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "10", Some("1000")))
        .unwrap();
    quote(&mut v);
    let first = v
        .post_metal_movement(sale_input(&v, &owner, &lot, &bank))
        .unwrap();
    let old = v.list_metals_at(None).unwrap();
    let old_cash = v.list_treasury_at(None).unwrap();
    let t = old_cash
        .transactions
        .iter()
        .find(|t| t.kind == "asset_trade")
        .unwrap();
    let mut correction = sale_input(&v, &owner, &lot, &bank);
    correction.movement.id = Some(first.id.clone());
    correction.movement.expected_revision_id = Some(first.revision_id.clone());
    correction.movement.trade_amount = Some("270".into());
    correction.transaction = Some(SaveTreasuryTransactionInput {
        id: Some(t.id.clone()),
        expected_revision_id: Some(t.revision_id.clone()),
        entries: vec![TreasuryEntry {
            amount: "260".into(),
            ..t.entries[0].clone()
        }],
        ..transaction(&bank, "asset_trade", "2025-02-01", "260")
    });
    correction.links[0].amount = "260".into();
    let corrected = v.post_metal_movement(correction).unwrap();
    let current = v.list_metals_at(None).unwrap();
    assert_eq!(current.knowledge_sequence, old.knowledge_sequence + 1);
    assert_eq!(current.cash_links.len(), 1);
    assert_eq!(
        current.cash_links[0].movement_revision_id,
        corrected.revision_id
    );
    assert_eq!(v.list_treasury_at(None).unwrap().transactions.len(), 2);
    assert_eq!(bank_balance(&v, &bank), "1260.00");
    assert_eq!(
        report(&v, &lot, None).realized_gain_eur.as_deref(),
        Some("60.00")
    );
    assert_eq!(
        report(&v, &lot, Some(old.knowledge_sequence))
            .realized_gain_eur
            .as_deref(),
        Some("50.00")
    );
    assert_eq!(
        json!(v.list_metals_at(Some(old.knowledge_sequence)).unwrap()),
        json!(old)
    );
    let unchanged = state(&v);
    let mut isolated = movement(&lot, "sale", "2025-02-01", "-2", None);
    isolated.id = Some(corrected.id);
    isolated.expected_revision_id = Some(corrected.revision_id);
    isolated.trade_amount = Some("280".into());
    isolated.trade_currency = Some("EUR".into());
    isolated.cash_entity_id = owner.id;
    assert!(v.save_metal_movement(isolated).is_err());
    assert_eq!(state(&v), unchanged);
}

#[test]
fn stale_sequences_revisions_and_duplicate_bank_origins_roll_back_every_row() {
    let (_temp, mut v, owner, lot, bank) = fixture();
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "10", Some("1000")))
        .unwrap();
    let first = v
        .post_metal_movement(sale_input(&v, &owner, &lot, &bank))
        .unwrap();
    let other = v
        .save_treasury_transaction(transaction(&bank, "receipt", "2025-02-01", "250"))
        .unwrap();
    let unchanged = state(&v);
    let mut correction = sale_input(&v, &owner, &lot, &bank);
    correction.movement.id = Some(first.id);
    correction.movement.expected_revision_id = Some(first.revision_id);
    assert!(v.post_metal_movement(correction.clone()).is_err()); // second new bank origin
    assert_eq!(state(&v), unchanged);
    correction.transaction = None;
    correction.existing_transaction_revision_id = Some(other.revision_id);
    assert!(v.post_metal_movement(correction.clone()).is_err()); // another existing origin
    assert_eq!(state(&v), unchanged);
    let t = v
        .list_treasury_at(None)
        .unwrap()
        .transactions
        .into_iter()
        .find(|t| t.kind == "asset_trade")
        .unwrap();
    correction.existing_transaction_revision_id = Some(t.revision_id.clone());
    correction.expected_sequence -= 1;
    assert!(v.post_metal_movement(correction.clone()).is_err());
    assert_eq!(state(&v), unchanged);
    correction.expected_sequence += 1;
    correction.movement.expected_revision_id = Some("00000000-0000-4000-8000-000000000000".into());
    assert!(v.post_metal_movement(correction).is_err());
    assert_eq!(state(&v), unchanged);
    let mut ledger = transaction(&bank, "asset_trade", "2025-02-01", "260");
    ledger.id = Some(t.id);
    ledger.expected_revision_id = Some(t.revision_id);
    assert!(v.save_treasury_transaction(ledger).is_err()); // exact source link would become stale
    assert_eq!(state(&v), unchanged);
}

#[test]
fn shortages_and_structural_changes_are_rejected_without_inventory_or_cash_mutations() {
    let (_temp, mut v, owner, lot, bank) = fixture();
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "1", Some("100")))
        .unwrap();
    let unchanged = state(&v);
    assert!(v
        .post_metal_movement(sale_input(&v, &owner, &lot, &bank))
        .is_err());
    assert_eq!(state(&v), unchanged);
    let a = v
        .list_registry()
        .unwrap()
        .assets
        .into_iter()
        .find(|a| a.id == lot.asset_id)
        .unwrap();
    let mut input = lot_input(&a, &owner);
    input.id = Some(lot.id.clone());
    input.expected_revision_id = Some(lot.revision_id.clone());
    input.purity = "0.99".into();
    assert!(v.save_metal_lot(input).is_err());
    assert_eq!(state(&v), unchanged);
    let mut post = sale_input(&v, &owner, &lot, &bank);
    post.movement.quantity_change = "-1".into();
    post.links[0].amount = "200".into();
    post.transaction = Some(transaction(&bank, "asset_trade", "2025-02-01", "200"));
    assert!(v.post_metal_movement(post).is_err()); // partially linked source rejected by atomic API
    assert_eq!(state(&v), unchanged);
}

#[test]
fn storage_transfer_changes_location_without_changing_quantity_cost_or_cash() {
    let (_temp, mut v, _owner, lot, bank) = fixture();
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "2", Some("500")))
        .unwrap();
    quote(&mut v);
    let mut transfer = movement(&lot, "storage_transfer", "2025-03-01", "0", None);
    transfer.country = "CH".into();
    transfer.storage_location = "Coffre B".into();
    v.save_metal_movement(transfer).unwrap();
    let r = report(&v, &lot, None);
    assert_eq!(r.country, "CH");
    assert_eq!(r.storage_location, "Coffre B");
    assert_eq!(r.exact_quantity, fraction("2", "1"));
    assert_eq!(r.cost_remaining_original.as_deref(), Some("500.00"));
    assert_eq!(bank_balance(&v, &bank), "1000.00");
    let unchanged = state(&v);
    assert!(v
        .save_metal_movement(movement(
            &lot,
            "inventory_adjustment",
            "2025-04-01",
            "-1",
            None
        ))
        .is_err()); // old location
    assert_eq!(state(&v), unchanged);
}

fn transfer_input(v: &Vault, lot: &MetalLot, bob: &Entity) -> TransferMetalOwnershipInput {
    let old = v
        .list_registry()
        .unwrap()
        .ownerships
        .into_iter()
        .find(|o| o.asset_id == lot.asset_id)
        .unwrap();
    TransferMetalOwnershipInput {
        expected_sequence: v.list_metals_at(None).unwrap().knowledge_sequence,
        lot_id: lot.id.clone(),
        effective_on: "2025-03-01".into(),
        reason: "Transfert interne à droits conservés".into(),
        ownerships: vec![
            SaveOwnershipInput {
                id: Some(old.id),
                expected_revision_id: Some(old.revision_id),
                reason: "Clôture".into(),
                asset_id: lot.asset_id.clone(),
                owner_entity_id: old.owner_entity_id.clone(),
                legal_share: old.legal_share,
                economic_share: old.economic_share,
                valid_from: old.valid_from,
                valid_to: Some("2025-03-01".into()),
            },
            SaveOwnershipInput {
                id: None,
                expected_revision_id: None,
                reason: "Quote-part conservée".into(),
                asset_id: lot.asset_id.clone(),
                owner_entity_id: old.owner_entity_id,
                legal_share: fraction("1", "2"),
                economic_share: fraction("1", "2"),
                valid_from: "2025-03-01".into(),
                valid_to: None,
            },
            SaveOwnershipInput {
                id: None,
                expected_revision_id: None,
                reason: "Quote-part transférée".into(),
                asset_id: lot.asset_id.clone(),
                owner_entity_id: bob.id.clone(),
                legal_share: fraction("1", "2"),
                economic_share: fraction("1", "2"),
                valid_from: "2025-03-01".into(),
                valid_to: None,
            },
        ],
    }
}

#[test]
fn whole_or_partial_ownership_transfers_are_atomic_and_do_not_create_inventory_or_cash() {
    let (_temp, mut v, _owner, lot, bank) = fixture();
    let bob = person(&mut v, "Benoît synthétique");
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "2", Some("500")))
        .unwrap();
    quote(&mut v);
    let input = transfer_input(&v, &lot, &bob);
    let old = v.list_registry().unwrap();
    let old_k = old.knowledge_sequence;
    let old_metal = report(&v, &lot, None);
    let result = v.transfer_metal_ownership(input).unwrap();
    assert_eq!(result.knowledge_sequence, old_k + 1);
    assert_eq!(
        result
            .ownerships
            .iter()
            .filter(|o| o.asset_id == lot.asset_id && o.valid_from == "2025-03-01")
            .count(),
        2
    );
    assert_eq!(json!(v.list_registry_at(Some(old_k)).unwrap()), json!(old));
    let current = report(&v, &lot, None);
    assert_eq!(current.exact_quantity, old_metal.exact_quantity);
    assert_eq!(current.exact_valuation_eur, old_metal.exact_valuation_eur);
    assert_eq!(bank_balance(&v, &bank), "1000.00");
}

#[test]
fn nonconserving_or_stale_ownership_batches_leave_the_old_rights_intact() {
    let (_temp, mut v, _owner, lot, _bank) = fixture();
    let bob = person(&mut v, "Benoît synthétique");
    let mut input = transfer_input(&v, &lot, &bob);
    let unchanged = state(&v);
    input.ownerships[2].economic_share = fraction("1", "3");
    assert!(v.transfer_metal_ownership(input).is_err());
    assert_eq!(state(&v), unchanged);
    let mut input = transfer_input(&v, &lot, &bob);
    input.ownerships[0].expected_revision_id = Some("00000000-0000-4000-8000-000000000000".into());
    assert!(v.transfer_metal_ownership(input).is_err());
    assert_eq!(state(&v), unchanged);
}

#[test]
fn backed_up_corrected_quotes_trades_and_exact_links_reproduce_current_and_old_reports() {
    let (temp, mut v, owner, lot, bank) = fixture();
    v.save_metal_movement(movement(&lot, "opening", "2025-01-01", "10", Some("1000")))
        .unwrap();
    let old_quote = quote(&mut v);
    v.post_metal_movement(sale_input(&v, &owner, &lot, &bank))
        .unwrap();
    let old_k = v.list_metals_at(None).unwrap().knowledge_sequence;
    let old_report = report(&v, &lot, Some(old_k));
    v.save_metal_quote(SaveMetalQuoteInput {
        id: Some(old_quote.id),
        expected_revision_id: Some(old_quote.revision_id),
        reason: "Cours corrigé".into(),
        metal: "gold".into(),
        effective_on: "2025-01-01".into(),
        price: "61".into(),
        currency: "EUR".into(),
        weight_unit: "g".into(),
        source: "Correction synthétique".into(),
        notes: String::new(),
    })
    .unwrap();
    let before = state(&v);
    let current = report(&v, &lot, None);
    let path = temp.path().join("metals.zip");
    v.create_backup(&path).unwrap();
    let inspection = v.inspect_restore(&path).unwrap();
    assert_eq!(inspection.schema_version, 6);
    v.activate_restore(&inspection.token, true, None).unwrap();
    assert_eq!(state(&v), before);
    assert_eq!(json!(report(&v, &lot, None)), json!(current));
    assert_eq!(json!(report(&v, &lot, Some(old_k))), json!(old_report));
    assert_eq!(bank_balance(&v, &bank), "1250.00");
}
