//! Typed physical-lot histories and atomic trading/ownership operations.
use crate::{db, files::*, finance, phase7, properties, registry, treasury, Vault};
use patrimoine_application::MetalService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, TransactionBehavior};
use std::collections::BTreeSet;

pub(crate) const TABLES: [(&str, &str); 5] = [
    ("metal_lot", "metal_lots"),
    ("metal_movement", "metal_movements"),
    ("metal_quote", "metal_quotes"),
    ("metal_premium", "metal_premiums"),
    ("metal_cash_link", "metal_cash_links"),
];

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<MetalSnapshot> {
    let k = phase7::cutoff(c, cutoff)?;
    if !phase7::enabled(c)? {
        return Ok(MetalSnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    Ok(MetalSnapshot {
        lots: registry::load(c,"metal_lots","v.asset_id,v.metal,v.form,v.designation,v.unit_weight,v.weight_unit,v.purity,v.tracking_start,v.cost_currency,v.legal_owner_entity_id,v.country,v.storage_location,v.valuation_basis,v.notes,v.archived","",k,|r|Ok(MetalLot {id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,metal:r.get(3)?,form:r.get(4)?,designation:r.get(5)?,unit_weight:r.get(6)?,weight_unit:r.get(7)?,purity:r.get(8)?,tracking_start:r.get(9)?,cost_currency:r.get(10)?,legal_owner_entity_id:r.get(11)?,country:r.get(12)?,storage_location:r.get(13)?,valuation_basis:r.get(14)?,notes:r.get(15)?,archived:r.get(16)?}))?,
        movements: registry::load(c,"metal_movements","v.lot_id,v.kind,v.effective_on,v.quantity_change,v.cost_amount,v.trade_amount,v.trade_fees,v.trade_currency,v.cash_entity_id,v.counterparty_entity_id,v.country,v.storage_location,v.unlinked_reason,v.notes","",k,|r|Ok(MetalMovement {id:r.get(0)?,revision_id:r.get(1)?,lot_id:r.get(2)?,kind:r.get(3)?,effective_on:r.get(4)?,quantity_change:r.get(5)?,cost_amount:r.get(6)?,trade_amount:r.get(7)?,trade_fees:r.get(8)?,trade_currency:r.get(9)?,cash_entity_id:r.get(10)?,counterparty_entity_id:r.get(11)?,country:r.get(12)?,storage_location:r.get(13)?,unlinked_reason:r.get(14)?,notes:r.get(15)?}))?,
        quotes: registry::load(c,"metal_quotes","v.metal,v.effective_on,v.price,v.currency,v.weight_unit,v.source,v.notes","",k,|r|Ok(MetalQuote {id:r.get(0)?,revision_id:r.get(1)?,metal:r.get(2)?,effective_on:r.get(3)?,price:r.get(4)?,currency:r.get(5)?,weight_unit:r.get(6)?,source:r.get(7)?,notes:r.get(8)?}))?,
        premiums: registry::load(c,"metal_premiums","v.lot_id,v.effective_on,v.kind,v.value,v.currency,v.discount_per_unit,v.source,v.notes","",k,|r|Ok(MetalPremium {id:r.get(0)?,revision_id:r.get(1)?,lot_id:r.get(2)?,effective_on:r.get(3)?,kind:r.get(4)?,value:r.get(5)?,currency:r.get(6)?,discount_per_unit:r.get(7)?,source:r.get(8)?,notes:r.get(9)?}))?,
        cash_links: registry::load(c,"metal_cash_links","v.movement_id,v.movement_revision_id,v.transaction_revision_id,v.line_id,v.amount,v.notes","",k,|r|Ok(MetalCashLink {id:r.get(0)?,revision_id:r.get(1)?,movement_id:r.get(2)?,movement_revision_id:r.get(3)?,transaction_revision_id:r.get(4)?,line_id:r.get(5)?,amount:r.get(6)?,notes:r.get(7)?}))?,
        knowledge_sequence:k,
    })
}

fn normalize_movement(input: &mut SaveMetalMovementInput) -> Result<()> {
    input.quantity_change = exact::normalize_decimal_input(&input.quantity_change)?;
    input.trade_fees = exact::normalize_decimal_input(&input.trade_fees)?;
    for amount in [&mut input.cost_amount, &mut input.trade_amount]
        .into_iter()
        .flatten()
    {
        *amount = exact::normalize_decimal_input(amount)?;
    }
    Ok(())
}
fn insert_movement(c: &Connection, revision: &str, input: &SaveMetalMovementInput) -> Result<()> {
    c.execute(
        "INSERT INTO metal_movements VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
        params![
            revision,
            input.lot_id,
            input.kind,
            input.effective_on,
            input.quantity_change,
            input.cost_amount,
            input.trade_amount,
            input.trade_fees,
            input.trade_currency,
            input.cash_entity_id,
            input.counterparty_entity_id,
            input.country,
            input.storage_location,
            input.unlinked_reason,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}
fn insert_link(c: &Connection, revision: &str, input: &SaveMetalCashLinkInput) -> Result<()> {
    c.execute(
        "INSERT INTO metal_cash_links VALUES(?1,?2,?3,?4,?5,?6,?7)",
        params![
            revision,
            input.movement_id,
            input.movement_revision_id,
            input.transaction_revision_id,
            input.line_id,
            input.amount,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}
fn immutable_movement(c: &Connection, input: &SaveMetalMovementInput) -> Result<()> {
    for (column, value) in [
        ("lot_id", input.lot_id.as_str()),
        ("kind", input.kind.as_str()),
    ] {
        phase7::unchanged(c, "metal_movements", column, input.id.as_deref(), value)?;
    }
    Ok(())
}
pub(crate) fn validate_targets(c: &Connection) -> Result<()> {
    if !phase7::enabled(c)? {
        return Ok(());
    }
    phase7::immutable_history(
        c,
        "metal_lots",
        &[
            "asset_id",
            "metal",
            "unit_weight",
            "weight_unit",
            "purity",
            "tracking_start",
            "cost_currency",
        ],
    )?;
    phase7::immutable_history(c, "metal_movements", &["lot_id", "kind"])?;
    phase7::immutable_history(c, "metal_quotes", &["metal"])?;
    phase7::immutable_history(c, "metal_premiums", &["lot_id"])?;
    phase7::immutable_history(c, "metal_cash_links", &["movement_id"])
}
pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    if !phase7::enabled(c)? {
        return Ok(());
    }
    validate_targets(c)?;
    finance::validate_record_history(c, &TABLES)
}

impl MetalService for Vault {
    fn list_metals_at(&self, knowledge_sequence: Option<u64>) -> Result<MetalSnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_metal_lot(&mut self, mut input: SaveMetalLotInput) -> Result<MetalLot> {
        input.unit_weight = exact::normalize_decimal_input(&input.unit_weight)?;
        input.purity = exact::normalize_decimal_input(&input.purity)?;
        let id=self.write_phase7("metal_lot",input.id.as_deref(),input.expected_revision_id.as_deref(),&input.reason,|tx,_,revision| {
            for (column,value) in [("asset_id",input.asset_id.as_str()),("metal",input.metal.as_str()),("unit_weight",input.unit_weight.as_str()),("weight_unit",input.weight_unit.as_str()),("purity",input.purity.as_str()),("tracking_start",input.tracking_start.as_str()),("cost_currency",input.cost_currency.as_str())] { phase7::unchanged(tx,"metal_lots",column,input.id.as_deref(),value)?; }
            tx.execute("INSERT INTO metal_lots VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![revision,input.asset_id,input.metal,input.form,input.designation,input.unit_weight,input.weight_unit,input.purity,input.tracking_start,input.cost_currency,input.legal_owner_entity_id,input.country,input.storage_location,input.valuation_basis,input.notes,input.archived]).map_err(db::error)?;
            Ok(())
        })?;
        self.list_metals_at(None)?
            .lots
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| invalid("Le lot enregistré est introuvable."))
    }
    fn save_metal_movement(&mut self, mut input: SaveMetalMovementInput) -> Result<MetalMovement> {
        normalize_movement(&mut input)?;
        let id = self.write_phase7(
            "metal_movement",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                immutable_movement(tx, &input)?;
                insert_movement(tx, revision, &input)
            },
        )?;
        self.list_metals_at(None)?
            .movements
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| invalid("Le mouvement enregistré est introuvable."))
    }
    fn save_metal_quote(&mut self, mut input: SaveMetalQuoteInput) -> Result<MetalQuote> {
        input.price = exact::normalize_decimal_input(&input.price)?;
        let id = self.write_phase7(
            "metal_quote",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                phase7::unchanged(
                    tx,
                    "metal_quotes",
                    "metal",
                    input.id.as_deref(),
                    &input.metal,
                )?;
                tx.execute(
                    "INSERT INTO metal_quotes VALUES(?1,?2,?3,?4,?5,?6,?7,?8)",
                    params![
                        revision,
                        input.metal,
                        input.effective_on,
                        input.price,
                        input.currency,
                        input.weight_unit,
                        input.source,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_metals_at(None)?
            .quotes
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| invalid("Le cours enregistré est introuvable."))
    }
    fn save_metal_premium(&mut self, mut input: SaveMetalPremiumInput) -> Result<MetalPremium> {
        input.value = exact::normalize_decimal_input(&input.value)?;
        input.discount_per_unit = exact::normalize_decimal_input(&input.discount_per_unit)?;
        let id = self.write_phase7(
            "metal_premium",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                phase7::unchanged(
                    tx,
                    "metal_premiums",
                    "lot_id",
                    input.id.as_deref(),
                    &input.lot_id,
                )?;
                tx.execute(
                    "INSERT INTO metal_premiums VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                    params![
                        revision,
                        input.lot_id,
                        input.effective_on,
                        input.kind,
                        input.value,
                        input.currency,
                        input.discount_per_unit,
                        input.source,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
        )?;
        self.list_metals_at(None)?
            .premiums
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| invalid("La prime enregistrée est introuvable."))
    }
    fn save_metal_cash_link(&mut self, mut input: SaveMetalCashLinkInput) -> Result<MetalCashLink> {
        input.amount = exact::normalize_decimal_input(&input.amount)?;
        let id=self.write_phase7("metal_cash_link",input.id.as_deref(),input.expected_revision_id.as_deref(),&input.reason,|tx,_,revision| {
            phase7::unchanged(tx,"metal_cash_links","movement_id",input.id.as_deref(),&input.movement_id)?;
            if let Some(parent)=&input.expected_revision_id {
                let old:String=tx.query_row("SELECT transaction_revision_id FROM metal_cash_links WHERE revision_id=?1",[parent],|r|r.get(0)).map_err(db::error)?;
                phase7::preserve_cash_origins(tx,std::slice::from_ref(&old),std::slice::from_ref(&input.transaction_revision_id))?;
            }
            insert_link(tx,revision,&input)
        })?;
        self.list_metals_at(None)?
            .cash_links
            .into_iter()
            .find(|m| m.id == id)
            .ok_or_else(|| invalid("Le lien enregistré est introuvable."))
    }
    fn post_metal_movement(&mut self, mut input: PostMetalMovementInput) -> Result<MetalMovement> {
        self.ensure_active()?;
        normalize_movement(&mut input.movement)?;
        if !["purchase", "sale"].contains(&input.movement.kind.as_str()) || input.links.is_empty() {
            return Err(invalid(
                "Un achat ou une vente réel désigne ses lignes de trésorerie explicites.",
            ));
        }
        for link in &mut input.links {
            link.amount = exact::normalize_decimal_input(&link.amount)?;
        }
        if input.movement.unlinked_reason.trim().is_empty() {
            input.movement.unlinked_reason =
                "Mouvement rapproché avec ses lignes de trésorerie réelles.".into();
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        phase7::expect_sequence(&tx, input.expected_sequence)?;
        immutable_movement(&tx, &input.movement)?;
        let before = snapshot(&tx, None)?;
        let old_links = before
            .cash_links
            .iter()
            .filter(|l| Some(&l.movement_id) == input.movement.id.as_ref())
            .cloned()
            .collect::<Vec<_>>();
        let old_revisions = old_links
            .iter()
            .map(|l| l.transaction_revision_id.clone())
            .collect::<Vec<_>>();
        let (change, stamp) = db::mutation(&tx, input.movement.reason.trim())?;
        let transaction_revision = phase7::resolve_cash(
            &tx,
            &mut input.transaction,
            &input.existing_transaction_revision_id,
            &old_revisions,
            &input.movement.reason,
            (&change, &stamp),
        )?;
        let (id, revision) = registry::begin_shared(
            &tx,
            input.movement.id.as_deref(),
            input.movement.expected_revision_id.as_deref(),
            "metal_movement",
            &input.movement.reason,
            false,
            Some((&change, &stamp)),
        )?;
        insert_movement(&tx, &revision, &input.movement)?;
        for old in &old_links {
            let (_, void_revision) = registry::begin_shared(
                &tx,
                Some(&old.id),
                Some(&old.revision_id),
                "metal_cash_link",
                &input.movement.reason,
                true,
                Some((&change, &stamp)),
            )?;
            tx.execute("INSERT INTO metal_cash_links SELECT ?1,movement_id,movement_revision_id,transaction_revision_id,line_id,amount,notes FROM metal_cash_links WHERE revision_id=?2",params![void_revision,old.revision_id]).map_err(db::error)?;
        }
        for link in input.links {
            let (_, link_revision) = registry::begin_shared(
                &tx,
                None,
                None,
                "metal_cash_link",
                &input.movement.reason,
                false,
                Some((&change, &stamp)),
            )?;
            insert_link(
                &tx,
                &link_revision,
                &SaveMetalCashLinkInput {
                    id: None,
                    expected_revision_id: None,
                    reason: input.movement.reason.clone(),
                    movement_id: id.clone(),
                    movement_revision_id: revision.clone(),
                    transaction_revision_id: transaction_revision.clone(),
                    line_id: link.line_id,
                    amount: link.amount,
                    notes: "Rapprochement atomique.".into(),
                },
            )?;
        }
        finance::validate_current(&tx)?;
        let state = snapshot(&tx, None)?;
        let result = state
            .movements
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or_else(|| invalid("Le mouvement rapproché est introuvable."))?;
        if !metal_cash_coverage(&state, &result)? {
            return Err(invalid("Le rapprochement atomique doit couvrir entièrement le prix net et les frais du mouvement métal."));
        }
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn transfer_metal_ownership(
        &mut self,
        mut input: TransferMetalOwnershipInput,
    ) -> Result<RegistrySnapshot> {
        self.ensure_active()?;
        validate_date(&input.effective_on)?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        phase7::expect_sequence(&tx, input.expected_sequence)?;
        let metals = snapshot(&tx, None)?;
        let lot = metals
            .lots
            .iter()
            .find(|l| l.id == input.lot_id)
            .ok_or_else(|| invalid("Le lot à transférer est inconnu."))?;
        if input.effective_on < lot.tracking_start || input.ownerships.is_empty() {
            return Err(invalid(
                "Le transfert porte sur une date suivie et un ensemble explicite de droits.",
            ));
        }
        let before = registry::snapshot(&tx, None)?;
        let active = before
            .ownerships
            .iter()
            .filter(|o| {
                o.asset_id == lot.asset_id
                    && o.valid_from <= input.effective_on
                    && o.valid_to
                        .as_ref()
                        .is_none_or(|end| &input.effective_on < end)
            })
            .collect::<Vec<_>>();
        let closed = input
            .ownerships
            .iter()
            .filter_map(|o| o.id.as_ref())
            .cloned()
            .collect::<BTreeSet<_>>();
        if closed.len() != active.len() || active.iter().any(|o| !closed.contains(&o.id)) {
            return Err(invalid("Le transfert ferme explicitement tous les droits actifs avant d’ouvrir leur nouvelle répartition."));
        }
        let (change, stamp) = db::mutation(&tx, input.reason.trim())?;
        for row in &mut input.ownerships {
            if row.asset_id != lot.asset_id {
                return Err(invalid(
                    "Toutes les lignes du transfert appartiennent au même lot.",
                ));
            }
            for share in [&mut row.legal_share, &mut row.economic_share]
                .into_iter()
                .flatten()
            {
                *share = exact::normalize_fraction(share)?;
            }
            if let Some(id) = &row.id {
                let old = active
                    .iter()
                    .find(|o| &o.id == id)
                    .ok_or_else(|| invalid("Le droit à fermer n’est plus actif."))?;
                if row.owner_entity_id != old.owner_entity_id
                    || row.valid_from != old.valid_from
                    || row.valid_to.as_deref() != Some(input.effective_on.as_str())
                    || row.legal_share != old.legal_share
                    || row.economic_share != old.economic_share
                {
                    return Err(invalid("Un droit source est fermé à la date du transfert sans réécrire son titulaire ou sa fraction passée."));
                }
            } else if row.valid_from != input.effective_on {
                return Err(invalid(
                    "Les nouveaux droits commencent exactement à la date du transfert.",
                ));
            }
            let (_, revision) = registry::begin_shared(
                &tx,
                row.id.as_deref(),
                row.expected_revision_id.as_deref(),
                "ownership",
                &input.reason,
                false,
                Some((&change, &stamp)),
            )?;
            let (ln, ld) = row
                .legal_share
                .as_ref()
                .map(|v| (Some(v.num.as_str()), Some(v.den.as_str())))
                .unwrap_or((None, None));
            let (en, ed) = row
                .economic_share
                .as_ref()
                .map(|v| (Some(v.num.as_str()), Some(v.den.as_str())))
                .unwrap_or((None, None));
            tx.execute(
                "INSERT INTO asset_ownerships VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    revision,
                    row.asset_id,
                    row.owner_entity_id,
                    ln,
                    ld,
                    en,
                    ed,
                    row.valid_from,
                    row.valid_to
                ],
            )
            .map_err(db::error)?;
        }
        let after = registry::snapshot(&tx, None)?;
        let mut dates = BTreeSet::from([input.effective_on.clone()]);
        for row in before
            .ownerships
            .iter()
            .chain(after.ownerships.iter())
            .filter(|o| o.asset_id == lot.asset_id)
        {
            if row.valid_from >= input.effective_on {
                dates.insert(row.valid_from.clone());
            }
            if let Some(end) = &row.valid_to {
                if end >= &input.effective_on {
                    dates.insert(end.clone());
                }
            }
        }
        for date in dates {
            for legal in [false, true] {
                let total = |state: &RegistrySnapshot| -> Result<_> {
                    let mut sum = exact::parse_decimal("0")?;
                    for row in state.ownerships.iter().filter(|o| {
                        o.asset_id == lot.asset_id
                            && o.valid_from <= date
                            && o.valid_to.as_ref().is_none_or(|end| &date < end)
                    }) {
                        let share = if legal {
                            &row.legal_share
                        } else {
                            &row.economic_share
                        };
                        sum+=exact::parse_fraction(share.as_ref().ok_or_else(||invalid("Clarifiez les droits juridiques et économiques inconnus avant de transférer leur répartition."))?)?;
                    }
                    Ok(sum)
                };
                if total(&before)? != total(&after)? {
                    return Err(invalid("Le transfert conserve le total des droits juridiques et économiques à toutes les dates affectées."));
                }
            }
        }
        validate_registry(&after)?;
        finance::validate_current(&tx)?;
        tx.commit().map_err(db::error)?;
        Ok(after)
    }
    fn get_metal_report(&self, request: MetalReportRequest) -> Result<MetalReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let k = request.knowledge_sequence;
        metal_report(
            &registry::snapshot(&tx, k)?,
            &properties::snapshot(&tx, k)?,
            &treasury::snapshot(&tx, k)?,
            &snapshot(&tx, k)?,
            &request,
        )
    }
}
