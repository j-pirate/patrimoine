//! First entries reuse the established record writers in one SQLite change.
use crate::{db, files, metals, properties, registry, treasury, Vault};
use patrimoine_application::GuidedCreationService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

const RECEIPT_ACTION: &str = "guided_entry_created";
const ONBOARDING_LIMIT: usize = 512 * 1024;

fn invalid(message: &str) -> AppError {
    AppError::new(ErrorCode::Validation, message)
}

fn known_country(country: &str) -> Result<()> {
    if ["FR", "TN", "CH", "AE"].contains(&country) {
        return Ok(());
    }
    Err(invalid("Choisissez un pays proposé dans la liste."))
}

struct GuidedChange<'a> {
    c: &'a Connection,
    reason: &'a str,
    change: &'a str,
    stamp: &'a str,
}

impl GuidedChange<'_> {
    fn add(
        &self,
        kind: &str,
        insert: impl FnOnce(&Connection, &str, &str) -> Result<()>,
    ) -> Result<(String, String)> {
        let (object, revision) = registry::begin_shared(
            self.c,
            None,
            None,
            kind,
            self.reason,
            false,
            Some((self.change, self.stamp)),
        )?;
        insert(self.c, &object, &revision)?;
        Ok((object, revision))
    }

    fn asset(&self, name: &str, country: &str, category: &str) -> Result<(String, String)> {
        known_country(country)?;
        let input = SaveAssetInput {
            id: None,
            expected_revision_id: None,
            reason: self.reason.into(),
            name: name.into(),
            country: country.into(),
            category: category.into(),
            notes: String::new(),
            archived: false,
        };
        self.add("asset", |c, key, rev| {
            registry::insert_asset(c, key, rev, &input)
        })
    }

    fn require_active_owner(&self, owner: &str) -> Result<()> {
        let exists = self.c.query_row(
            "SELECT EXISTS(SELECT 1 FROM entity_profiles p JOIN revisions r ON r.id=p.revision_id
             WHERE p.entity_id=?1 AND p.archived=0 AND r.is_void=0
             AND NOT EXISTS(SELECT 1 FROM revisions n WHERE n.supersedes_revision_id=r.id))",
            [owner],
            |row| row.get::<_, bool>(0),
        ).map_err(db::error)?;
        if exists {
            return Ok(());
        }
        Err(invalid(
            "Choisissez un propriétaire déjà enregistré et actif.",
        ))
    }

    fn sole_owner(&self, asset: &str, owner: &str, date: &str) -> Result<()> {
        self.require_active_owner(owner)?;
        let full = Fraction {
            num: "1".into(),
            den: "1".into(),
        };
        let input = SaveOwnershipInput {
            id: None,
            expected_revision_id: None,
            reason: self.reason.into(),
            asset_id: asset.into(),
            owner_entity_id: owner.into(),
            legal_share: Some(full.clone()),
            economic_share: Some(full),
            valid_from: date.into(),
            valid_to: None,
        };
        self.add("ownership", |c, _, rev| {
            registry::insert_ownership(c, rev, &input)
        })?;
        Ok(())
    }
}

impl Vault {
    fn onboarding_path(&self, generation_id: &str) -> Result<PathBuf> {
        self.ensure_active()?;
        if generation_id != self.generation {
            return Err(AppError::new(
                ErrorCode::Conflict,
                "Le coffre actif a changé. Rouvrez la configuration pour reprendre la saisie.",
            ));
        }
        let path = self.generation_path().join("onboarding.json");
        files::no_symlinks(&path)?;
        Ok(path)
    }

    fn write_guided(
        &mut self,
        input: &impl Serialize,
        request_id: &str,
        date: &str,
        reason: &str,
        name: &str,
        write: impl FnOnce(&GuidedChange<'_>) -> Result<(String, String)>,
    ) -> Result<GuidedRecord> {
        self.ensure_active()?;
        if !files::valid_id(request_id) {
            return Err(invalid(
                "La tentative de saisie est invalide. Rouvrez le formulaire.",
            ));
        }
        validate_date(date)?;
        let payload = serde_json::to_vec(&(reason, input))
            .map_err(|_| invalid("Les informations saisies ne peuvent pas être enregistrées."))?;
        let fingerprint = files::hash_bytes(&payload);
        let mut connection = db::connect(&self.db_path())?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;

        // The command UUID and the receipt commit with the records. A retry after
        // closing WebKit can recover the original result without writing again.
        let previous: Option<(Option<String>, Option<String>)> = tx
            .query_row(
                "SELECT a.object_id,a.summary FROM change_sets cs
             LEFT JOIN audit_events a ON a.change_set_id=cs.id AND a.action=?2
             WHERE cs.command_id=?1",
                params![request_id, RECEIPT_ACTION],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(db::error)?;
        if let Some((saved_id, saved_fingerprint)) = previous {
            if let Some(id) =
                saved_id.filter(|_| saved_fingerprint.as_deref() == Some(&fingerprint))
            {
                return Ok(GuidedRecord {
                    id,
                    name: name.into(),
                });
            }
            return Err(AppError::new(ErrorCode::Conflict, "Cette tentative a déjà enregistré d’autres informations. Rouvrez le formulaire pour une nouvelle saisie."));
        }

        let (change, stamp) = db::mutation_with_command(&tx, reason, request_id)?;
        let context = GuidedChange {
            c: &tx,
            reason,
            change: &change,
            stamp: &stamp,
        };
        let (id, revision) = write(&context)?;
        registry::validate_current(&tx)?;
        properties::validate_identities(&tx)?;
        tx.execute(
            "INSERT INTO audit_events VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                files::id(),
                change,
                id,
                revision,
                RECEIPT_ACTION,
                fingerprint
            ],
        )
        .map_err(db::error)?;
        tx.commit().map_err(db::error)?;
        Ok(GuidedRecord {
            id,
            name: name.into(),
        })
    }
}

impl GuidedCreationService for Vault {
    fn read_onboarding_state(&self, generation_id: &str) -> Result<Option<String>> {
        let path = self.onboarding_path(generation_id)?;
        if !path.try_exists().map_err(files::io)? {
            return Ok(None);
        }
        let mut data = String::new();
        files::read_file(&path)?
            .take((ONBOARDING_LIMIT + 1) as u64)
            .read_to_string(&mut data)
            .map_err(files::io)?;
        validate_onboarding_data(&data)?;
        Ok(Some(data))
    }

    fn write_onboarding_state(&self, generation_id: &str, data: &str) -> Result<()> {
        let path = self.onboarding_path(generation_id)?;
        validate_onboarding_data(data)?;
        let directory = self.generation_path();
        let staging = directory.join(format!(".onboarding-{}", files::id()));
        let result = (|| {
            let mut file = files::new_file(&staging)?;
            file.write_all(data.as_bytes()).map_err(files::io)?;
            file.sync_all().map_err(files::io)?;
            fs::rename(&staging, path).map_err(files::io)?;
            files::sync_dir(&directory)
        })();
        if result.is_err() {
            let _ = fs::remove_file(staging);
        }
        result
    }

    fn create_guided_family(&mut self, input: GuidedFamilyInput) -> Result<GuidedRecord> {
        let first = input.first_name.trim();
        let last = input.last_name.trim();
        let name = format!("{first} {last}");
        let reason = "Ajout d’un membre de la famille";
        self.write_guided(
            &input,
            &input.request_id,
            &input.date,
            reason,
            &name,
            |ctx| {
                if first.is_empty() || last.is_empty() {
                    return Err(invalid(
                        "Indiquez le prénom et le nom du membre de la famille.",
                    ));
                }
                if let Some(country) = input.residence_country.as_deref() {
                    known_country(country)?;
                }
                let person = SaveEntityInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    kind: "person".into(),
                    display_name: name.clone(),
                    first_name: Some(first.into()),
                    last_name: Some(last.into()),
                    birth_date: None,
                    residence_country: input.residence_country.clone(),
                    preferred_currency: db::settings(ctx.c)?.display_currency,
                    activity: String::new(),
                    notes: String::new(),
                    archived: false,
                };
                let result = ctx.add("entity", |c, key, rev| {
                    registry::insert_entity(c, key, rev, &person)
                })?;
                let membership = SaveMembershipInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    person_id: result.0.clone(),
                    valid_from: input.date.clone(),
                    valid_to: None,
                    relationship: String::new(),
                };
                ctx.add("membership", |c, _, rev| {
                    registry::insert_membership(c, rev, &membership)
                })?;
                Ok(result)
            },
        )
    }

    fn create_guided_property(&mut self, input: GuidedPropertyInput) -> Result<GuidedRecord> {
        let name = input.name.trim();
        let reason = "Ajout d’un bien immobilier";
        self.write_guided(
            &input,
            &input.request_id,
            &input.date,
            reason,
            name,
            |ctx| {
                if ![
                    "owner_occupied",
                    "rented",
                    "vacant",
                    "renovation",
                    "for_sale",
                ]
                .contains(&input.status.as_str())
                {
                    return Err(invalid("Choisissez la situation actuelle du bien."));
                }
                let result = ctx.asset(name, &input.country, "immobilier")?;
                let details = SavePropertyInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    asset_id: result.0.clone(),
                    property_type: input.property_type.clone(),
                    city: input.city.trim().into(),
                    address: String::new(),
                    surface_m2: None,
                    status: input.status.clone(),
                    valid_from: input.date.clone(),
                    valid_to: None,
                    notes: String::new(),
                };
                ctx.add("property", |c, _, rev| {
                    properties::insert_property(c, rev, &details)
                })?;
                if let Some(owner) = input.sole_owner_entity_id.as_deref() {
                    ctx.sole_owner(&result.0, owner, &input.date)?;
                }
                if let Some(amount) = &input.estimated_value {
                    exact::currency_minor_units(&input.currency)?;
                    let valuation = SaveValuationInput {
                        id: None,
                        expected_revision_id: None,
                        reason: reason.into(),
                        target_kind: "asset".into(),
                        target_id: result.0.clone(),
                        amount: exact::normalize_decimal_input(amount)?,
                        currency: input.currency.clone(),
                        effective_on: input.date.clone(),
                        source: "Estimation saisie par l’utilisateur".into(),
                    };
                    ctx.add("valuation", |c, _, rev| {
                        registry::insert_valuation(c, rev, &valuation)
                    })?;
                }
                Ok(result)
            },
        )
    }

    fn create_guided_account(&mut self, input: GuidedAccountInput) -> Result<GuidedRecord> {
        let name = input.name.trim();
        let reason = "Ajout d’un compte ou d’une caisse";
        self.write_guided(
            &input,
            &input.request_id,
            &input.date,
            reason,
            name,
            |ctx| {
                let bank_name = match input.kind.as_str() {
                    "bank" => name.to_owned(),
                    "cash" => String::new(),
                    _ => {
                        return Err(invalid(
                            "Choisissez un compte bancaire ou une caisse d’espèces.",
                        ))
                    }
                };
                exact::currency_minor_units(&input.currency)?;
                let (asset, _) = ctx.asset(name, &input.country, "liquidités")?;
                ctx.sole_owner(&asset, &input.owner_entity_id, &input.date)?;
                let account = SaveTreasuryAccountInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    asset_id: asset,
                    kind: input.kind.clone(),
                    currency: input.currency.clone(),
                    tracking_start: input.date.clone(),
                    legal_owner_entity_id: input.owner_entity_id.clone(),
                    bank_name,
                    reference: String::new(),
                    country: input.country.clone(),
                    storage_location: input.storage_location.trim().into(),
                    notes: String::new(),
                    archived: false,
                };
                let result = ctx.add("treasury_account", |c, _, rev| {
                    treasury::insert_account(c, rev, &account)
                })?;
                let mut opening = SaveTreasuryTransactionInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    kind: "opening".into(),
                    effective_on: input.date.clone(),
                    label: "Solde de départ".into(),
                    reference: String::new(),
                    notes: String::new(),
                    entries: vec![TreasuryEntry {
                        line_id: files::id(),
                        account_id: result.0.clone(),
                        amount: input.opening_balance.clone(),
                        role: "opening".into(),
                    }],
                };
                treasury::normalize_transaction(&mut opening)?;
                ctx.add("treasury_transaction", |c, _, rev| {
                    treasury::insert_transaction(c, rev, &opening)
                })?;
                Ok(result)
            },
        )
    }

    fn create_guided_metal(&mut self, input: GuidedMetalInput) -> Result<GuidedRecord> {
        let name = input.designation.trim();
        let reason = "Ajout d’un lot d’or ou d’argent";
        self.write_guided(
            &input,
            &input.request_id,
            &input.date,
            reason,
            name,
            |ctx| {
                exact::currency_minor_units(&input.cost_currency)?;
                let (asset, _) = ctx.asset(name, &input.country, "métaux")?;
                ctx.sole_owner(&asset, &input.owner_entity_id, &input.date)?;
                let mut lot = SaveMetalLotInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    asset_id: asset,
                    metal: input.metal.clone(),
                    form: input.form.clone(),
                    designation: name.into(),
                    unit_weight: input.unit_weight.clone(),
                    weight_unit: input.weight_unit.clone(),
                    purity: input.purity.clone(),
                    tracking_start: input.date.clone(),
                    cost_currency: input.cost_currency.clone(),
                    legal_owner_entity_id: input.owner_entity_id.clone(),
                    country: input.country.clone(),
                    storage_location: input.storage_location.trim().into(),
                    valuation_basis: "intrinsic".into(),
                    notes: String::new(),
                    archived: false,
                };
                metals::normalize_lot(&mut lot)?;
                let result = ctx.add("metal_lot", |c, _, rev| metals::insert_lot(c, rev, &lot))?;
                let mut opening = SaveMetalMovementInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    lot_id: result.0.clone(),
                    kind: "opening".into(),
                    effective_on: input.date.clone(),
                    quantity_change: input.quantity.clone(),
                    cost_amount: None,
                    trade_amount: None,
                    trade_fees: "0".into(),
                    trade_currency: None,
                    cash_entity_id: String::new(),
                    counterparty_entity_id: None,
                    country: input.country.clone(),
                    storage_location: lot.storage_location,
                    unlinked_reason: String::new(),
                    notes: String::new(),
                };
                metals::normalize_movement(&mut opening)?;
                ctx.add("metal_movement", |c, _, rev| {
                    metals::insert_movement(c, rev, &opening)
                })?;
                Ok(result)
            },
        )
    }

    fn create_guided_company(&mut self, input: GuidedCompanyInput) -> Result<GuidedRecord> {
        let name = input.name.trim();
        let reason = "Ajout d’une société";
        self.write_guided(
            &input,
            &input.request_id,
            &input.date,
            reason,
            name,
            |ctx| {
                known_country(&input.country)?;
                let company = SaveEntityInput {
                    id: None,
                    expected_revision_id: None,
                    reason: reason.into(),
                    kind: "company".into(),
                    display_name: name.into(),
                    first_name: None,
                    last_name: None,
                    birth_date: None,
                    residence_country: Some(input.country.clone()),
                    preferred_currency: db::settings(ctx.c)?.display_currency,
                    activity: input.activity.trim().into(),
                    notes: String::new(),
                    archived: false,
                };
                let result = ctx.add("entity", |c, key, rev| {
                    registry::insert_entity(c, key, rev, &company)
                })?;
                for owner in &input.owners {
                    ctx.require_active_owner(&owner.owner_entity_id)?;
                    let mut owner = SaveParticipationInput {
                        id: None,
                        expected_revision_id: None,
                        reason: reason.into(),
                        owner_entity_id: owner.owner_entity_id.clone(),
                        company_id: result.0.clone(),
                        legal_share: owner.legal_share.clone(),
                        economic_share: owner.economic_share.clone(),
                        voting_share: None,
                        valid_from: input.date.clone(),
                        valid_to: None,
                    };
                    registry::normalize_share(&mut owner.legal_share)?;
                    registry::normalize_share(&mut owner.economic_share)?;
                    ctx.add("participation", |c, _, rev| {
                        registry::insert_participation(c, rev, &owner)
                    })?;
                }
                Ok(result)
            },
        )
    }
}

fn validate_onboarding_data(data: &str) -> Result<()> {
    if data.len() > ONBOARDING_LIMIT {
        return Err(AppError::new(
            ErrorCode::Limit,
            "La configuration locale dépasse la taille autorisée.",
        ));
    }
    if !serde_json::from_str::<serde_json::Value>(data).is_ok_and(|value| value.is_object()) {
        return Err(invalid("La configuration locale ne peut pas être relue."));
    }
    Ok(())
}
