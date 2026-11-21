use patrimoine_application::{
    GuidedCreationService, MetalService, PropertyService, RegistryService, TreasuryService,
    VaultService,
};
use patrimoine_domain::*;
use patrimoine_infrastructure::Vault;
use std::fs::File;
use tempfile::TempDir;
use uuid::Uuid;

const DATE: &str = "2026-03-05";

fn request() -> String {
    Uuid::new_v4().to_string()
}

fn family() -> GuidedFamilyInput {
    GuidedFamilyInput {
        request_id: request(),
        date: DATE.into(),
        first_name: " Camille ".into(),
        last_name: "Exemple".into(),
        residence_country: None,
    }
}

fn property() -> GuidedPropertyInput {
    GuidedPropertyInput {
        request_id: request(),
        date: DATE.into(),
        name: "Appartement test".into(),
        property_type: "apartment".into(),
        country: "FR".into(),
        city: "Lyon".into(),
        status: "rented".into(),
        estimated_value: None,
        currency: "EUR".into(),
        sole_owner_entity_id: None,
    }
}

fn account(kind: &str, balance: &str, owner: &str) -> GuidedAccountInput {
    GuidedAccountInput {
        request_id: request(),
        date: DATE.into(),
        name: "Compte test".into(),
        kind: kind.into(),
        country: "FR".into(),
        currency: "USD".into(),
        opening_balance: balance.into(),
        owner_entity_id: owner.into(),
        storage_location: if kind == "cash" {
            "Caisse test".into()
        } else {
            String::new()
        },
    }
}

#[test]
fn first_property_from_the_empty_vault_keeps_unknowns_and_retries_by_request_after_reopening() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("vault");
    let mut vault = Vault::open(&path).unwrap();
    let input = property();
    let mut invalid = input.clone();
    invalid.estimated_value = Some("-123".into());
    assert_eq!(
        vault.create_guided_property(invalid).unwrap_err().code,
        "validation"
    );
    assert_eq!(vault.status().unwrap().mutation_sequence, 0);
    assert!(vault.list_registry().unwrap().assets.is_empty());

    let saved = vault.create_guided_property(input.clone()).unwrap();
    let details = vault.list_properties_at(None).unwrap();
    assert_eq!(details.properties[0].asset_id, saved.id);
    assert_eq!(details.properties[0].city, "Lyon");
    assert!(details.events.is_empty());
    assert!(details.units.is_empty());
    let registry = vault.list_registry().unwrap();
    assert_eq!(registry.assets.len(), 1);
    assert!(registry.ownerships.is_empty());
    assert!(registry.valuations.is_empty());
    assert!(registry.entities.is_empty());
    assert!(vault.list_registry_at(Some(0)).unwrap().assets.is_empty());
    drop(vault);

    let mut vault = Vault::open(&path).unwrap();
    assert_eq!(vault.create_guided_property(input.clone()).unwrap(), saved);
    let mut changed = input;
    changed.city = "Grenoble".into();
    assert_eq!(
        vault.create_guided_property(changed).unwrap_err().code,
        "conflict"
    );
    assert_eq!(vault.status().unwrap().mutation_sequence, 1);
    assert_eq!(vault.list_registry_history().unwrap().events.len(), 2);
}

#[test]
fn all_first_entries_use_normal_registry_and_module_reports() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("vault");
    let mut vault = Vault::open(&path).unwrap();
    let family_input = family();
    let person = vault.create_guided_family(family_input.clone()).unwrap();
    assert_eq!(person.name, "Camille Exemple");
    let registry = vault.list_registry().unwrap();
    assert_eq!(registry.memberships[0].person_id, person.id);
    assert_eq!(registry.entities[0].residence_country, None);

    let mut input = property();
    input.estimated_value = Some("150000,50".into());
    input.sole_owner_entity_id = Some(person.id.clone());
    let asset = vault.create_guided_property(input).unwrap();
    let bank = vault
        .create_guided_account(account("bank", "20,50", &person.id))
        .unwrap();
    let cash = vault
        .create_guided_account(account("cash", "0", &person.id))
        .unwrap();
    let lot = vault
        .create_guided_metal(GuidedMetalInput {
            request_id: request(),
            date: DATE.into(),
            designation: "Pièces test".into(),
            metal: "gold".into(),
            form: "coin".into(),
            quantity: "2".into(),
            unit_weight: "10".into(),
            weight_unit: "g".into(),
            purity: "0,9".into(),
            owner_entity_id: person.id.clone(),
            country: "FR".into(),
            storage_location: "Coffre test".into(),
            cost_currency: "EUR".into(),
        })
        .unwrap();
    let share = Fraction {
        num: "40".into(),
        den: "100".into(),
    };
    let company = vault
        .create_guided_company(GuidedCompanyInput {
            request_id: request(),
            date: DATE.into(),
            name: "Société test".into(),
            country: "CH".into(),
            activity: "Conseil".into(),
            owners: vec![GuidedCompanyOwnerInput {
                owner_entity_id: person.id.clone(),
                legal_share: None,
                economic_share: Some(share),
            }],
        })
        .unwrap();
    let registry = vault.list_registry().unwrap();
    assert_eq!(registry.knowledge_sequence, 6);
    assert_eq!(registry.assets.len(), 4);
    assert_eq!(registry.ownerships.len(), 4);
    assert!(registry
        .ownerships
        .iter()
        .all(|row| row.owner_entity_id == person.id));
    assert_eq!(registry.valuations[0].target_id, asset.id);
    assert_eq!(registry.valuations[0].amount, "150000.5");
    assert_eq!(registry.participations[0].company_id, company.id);
    assert_eq!(registry.participations[0].legal_share, None);
    assert_eq!(registry.participations[0].voting_share, None);
    assert_eq!(
        registry.participations[0].economic_share,
        Some(Fraction {
            num: "2".into(),
            den: "5".into()
        })
    );
    assert!(registry.company_policies.is_empty());
    assert!(registry.exchange_rates.is_empty());

    let report = vault
        .get_treasury_report(TreasuryReportRequest {
            account_id: bank.id,
            date: DATE.into(),
            knowledge_sequence: None,
        })
        .unwrap();
    assert_eq!(report.balance_original.as_deref(), Some("20.50"));
    assert_eq!(report.balance_eur, None);
    let report = vault
        .get_treasury_report(TreasuryReportRequest {
            account_id: cash.id,
            date: DATE.into(),
            knowledge_sequence: None,
        })
        .unwrap();
    assert_eq!(report.balance_original.as_deref(), Some("0.00"));
    assert!(report.opening_transaction_id.is_some());
    let report = vault
        .get_metal_report(MetalReportRequest {
            lot_id: lot.id,
            date: DATE.into(),
            knowledge_sequence: None,
        })
        .unwrap();
    assert_eq!(report.quantity_remaining.as_deref(), Some("2"));
    assert_eq!(report.fine_weight_g.as_deref(), Some("18"));
    assert_eq!(report.cost_remaining_original, None);
    assert_eq!(report.valuation_eur, None);
    assert!(report.quote.is_none());

    let before = vault.list_treasury_at(Some(2)).unwrap();
    assert!(before.accounts.is_empty());
    assert!(before.transactions.is_empty());
    let after = vault.list_treasury_at(Some(3)).unwrap();
    assert_eq!(after.accounts.len(), 1);
    assert_eq!(after.transactions.len(), 1);
    assert_eq!(vault.create_guided_family(family_input).unwrap(), person);
    drop(vault);
    assert_eq!(
        Vault::open(&path)
            .unwrap()
            .status()
            .unwrap()
            .mutation_sequence,
        6
    );
}

#[test]
fn an_invalid_later_entry_rolls_back_only_its_own_records() {
    let root = TempDir::new().unwrap();
    let mut vault = Vault::open(root.path().join("vault")).unwrap();
    let person = vault.create_guided_family(family()).unwrap();
    assert_eq!(
        vault
            .create_guided_account(account("cash", "-2", &person.id))
            .unwrap_err()
            .code,
        "validation"
    );
    assert_eq!(
        vault
            .create_guided_account(account("bank", "20", "unknown"))
            .unwrap_err()
            .code,
        "validation"
    );
    assert_eq!(
        vault
            .create_guided_company(GuidedCompanyInput {
                request_id: request(),
                date: DATE.into(),
                name: "Société test".into(),
                country: "CH".into(),
                activity: String::new(),
                owners: vec![GuidedCompanyOwnerInput {
                    owner_entity_id: person.id.clone(),
                    legal_share: None,
                    economic_share: Some(Fraction {
                        num: "11".into(),
                        den: "10".into(),
                    }),
                }],
            })
            .unwrap_err()
            .code,
        "validation"
    );
    let snapshot = vault.list_registry().unwrap();
    assert_eq!(snapshot.knowledge_sequence, 1);
    assert_eq!(snapshot.entities[0].id, person.id);
    assert_eq!(snapshot.entities.len(), 1);
    assert!(snapshot.assets.is_empty());
    assert!(snapshot.ownerships.is_empty());
    assert!(snapshot.participations.is_empty());
    assert!(vault.list_treasury_at(None).unwrap().accounts.is_empty());
    assert!(vault
        .list_treasury_at(None)
        .unwrap()
        .transactions
        .is_empty());
}

#[test]
fn a_known_company_associate_can_leave_its_shares_unknown() {
    let root = TempDir::new().unwrap();
    let mut vault = Vault::open(root.path().join("vault")).unwrap();
    let person = vault.create_guided_family(family()).unwrap();
    let company = vault
        .create_guided_company(GuidedCompanyInput {
            request_id: request(),
            date: DATE.into(),
            name: "Société test".into(),
            country: "CH".into(),
            activity: String::new(),
            owners: vec![GuidedCompanyOwnerInput {
                owner_entity_id: person.id.clone(),
                legal_share: None,
                economic_share: None,
            }],
        })
        .unwrap();
    let snapshot = vault.list_registry().unwrap();
    let row = &snapshot.participations[0];
    assert_eq!(row.company_id, company.id);
    assert_eq!(row.owner_entity_id, person.id);
    assert!(
        row.legal_share.is_none() && row.economic_share.is_none() && row.voting_share.is_none()
    );
    assert!(snapshot.company_policies.is_empty());
}

#[test]
fn the_configuration_draft_is_durable_locally_and_scoped_to_its_generation() {
    let temporary = TempDir::new().unwrap();
    let path = temporary.path().join("vault");
    let vault = Vault::open(&path).unwrap();
    let generation = vault.status().unwrap().generation_id;
    assert_eq!(vault.read_onboarding_state(&generation).unwrap(), None);
    let draft = r#"{"version":1,"step":2,"draft":"Saisie de test"}"#;
    vault.write_onboarding_state(&generation, draft).unwrap();
    assert_eq!(vault.status().unwrap().mutation_sequence, 0);
    assert_eq!(
        vault
            .write_onboarding_state(&generation, "{")
            .unwrap_err()
            .code,
        "validation"
    );
    assert_eq!(
        vault
            .write_onboarding_state(&generation, &" ".repeat(512 * 1024 + 1))
            .unwrap_err()
            .code,
        "limit"
    );
    drop(vault);

    let mut vault = Vault::open(&path).unwrap();
    assert_eq!(
        vault.read_onboarding_state(&generation).unwrap().as_deref(),
        Some(draft)
    );
    let archive = temporary.path().join("backup.zip");
    vault.create_backup(&archive).unwrap();
    let zipped = zip::ZipArchive::new(File::open(&archive).unwrap()).unwrap();
    assert!(zipped
        .file_names()
        .all(|name| !name.unwrap().contains("onboarding")));

    let candidate = vault.inspect_restore(&archive).unwrap();
    let prior = temporary.path().join("before-restore.zip");
    let restored = vault
        .activate_restore(&candidate.token, true, Some(&prior))
        .unwrap();
    assert_ne!(restored.generation_id, generation);
    assert_eq!(
        vault
            .read_onboarding_state(&restored.generation_id)
            .unwrap(),
        None
    );
    assert_eq!(
        vault.read_onboarding_state(&generation).unwrap_err().code,
        "conflict"
    );
    assert_eq!(
        vault
            .write_onboarding_state(&generation, draft)
            .unwrap_err()
            .code,
        "conflict"
    );
}
