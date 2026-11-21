//! Typed treasury persistence and atomic cross-module cash postings.
use crate::{db, debts, files::*, finance, properties, registry, rentals, Vault};
use patrimoine_application::TreasuryService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, Transaction, TransactionBehavior};

pub(crate) const TABLES: [(&str, &str); 5] = [
    ("treasury_account", "treasury_accounts"),
    ("treasury_transaction", "treasury_transactions"),
    ("account_reconciliation", "account_reconciliations"),
    ("cash_link", "cash_links"),
    ("deposit_movement", "deposit_movements"),
];

fn entries(c: &Connection, revision: &str) -> rusqlite::Result<Vec<TreasuryEntry>> {
    let mut q=c.prepare("SELECT line_id,account_id,amount,role FROM treasury_entries WHERE transaction_revision_id=?1 ORDER BY line_id")?;
    let rows = q
        .query_map([revision], |r| {
            Ok(TreasuryEntry {
                line_id: r.get(0)?,
                account_id: r.get(1)?,
                amount: r.get(2)?,
                role: r.get(3)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<TreasurySnapshot> {
    let latest = db::sequence(c)?;
    let k = cutoff.unwrap_or(latest);
    if k > latest {
        return Err(AppError::new(
            "validation",
            "La borne de connaissance demandée n’existe pas.",
        ));
    }
    if !finance::enabled(c)? {
        return Ok(TreasurySnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    Ok(TreasurySnapshot{
        accounts:registry::load(c,"treasury_accounts","v.asset_id,v.kind,v.currency,v.tracking_start,v.legal_owner_entity_id,v.bank_name,v.reference,v.country,v.storage_location,v.notes,v.archived","",k,|r|Ok(TreasuryAccount{id:r.get(0)?,revision_id:r.get(1)?,asset_id:r.get(2)?,kind:r.get(3)?,currency:r.get(4)?,tracking_start:r.get(5)?,legal_owner_entity_id:r.get(6)?,bank_name:r.get(7)?,reference:r.get(8)?,country:r.get(9)?,storage_location:r.get(10)?,notes:r.get(11)?,archived:r.get(12)?}))?,
        transactions:registry::load(c,"treasury_transactions","v.kind,v.effective_on,v.label,v.reference,v.notes","",k,|r|{let revision_id:String=r.get(1)?;Ok(TreasuryTransaction{id:r.get(0)?,entries:entries(c,&revision_id)?,revision_id,kind:r.get(2)?,effective_on:r.get(3)?,label:r.get(4)?,reference:r.get(5)?,notes:r.get(6)?})})?,
        reconciliations:registry::load(c,"account_reconciliations","v.account_id,v.effective_on,v.statement_balance,v.statement_reference,v.document_id,v.notes","",k,|r|Ok(AccountReconciliation{id:r.get(0)?,revision_id:r.get(1)?,account_id:r.get(2)?,effective_on:r.get(3)?,statement_balance:r.get(4)?,statement_reference:r.get(5)?,document_id:r.get(6)?,notes:r.get(7)?}))?,
        cash_links:registry::load(c,"cash_links","v.source_kind,v.source_id,v.source_revision_id,v.transaction_revision_id,v.line_id,v.amount,v.component,v.side,v.notes","",k,|r|Ok(CashLink{id:r.get(0)?,revision_id:r.get(1)?,source_kind:r.get(2)?,source_id:r.get(3)?,source_revision_id:r.get(4)?,transaction_revision_id:r.get(5)?,line_id:r.get(6)?,amount:r.get(7)?,component:r.get(8)?,side:r.get(9)?,notes:r.get(10)?}))?,
        deposits:registry::load(c,"deposit_movements","v.lease_id,v.kind,v.amount,v.currency,v.effective_on,v.recipient_entity_id,v.due_id,v.component,v.notes","",k,|r|Ok(DepositMovement{id:r.get(0)?,revision_id:r.get(1)?,lease_id:r.get(2)?,kind:r.get(3)?,amount:r.get(4)?,currency:r.get(5)?,effective_on:r.get(6)?,recipient_entity_id:r.get(7)?,due_id:r.get(8)?,component:r.get(9)?,notes:r.get(10)?}))?,
        knowledge_sequence:k,
    })
}

pub(crate) fn normalize_transaction(input: &mut SaveTreasuryTransactionInput) -> Result<()> {
    for entry in &mut input.entries {
        entry.amount = exact::normalize_decimal_input(&entry.amount)?;
    }
    Ok(())
}

pub(crate) fn insert_account(
    c: &Connection,
    revision: &str,
    input: &SaveTreasuryAccountInput,
) -> Result<()> {
    c.execute(
        "INSERT INTO treasury_accounts VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
        params![
            revision,
            input.asset_id,
            input.kind,
            input.currency,
            input.tracking_start,
            input.legal_owner_entity_id,
            input.bank_name,
            input.reference,
            input.country,
            input.storage_location,
            input.notes,
            input.archived
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

pub(crate) fn insert_transaction(
    c: &Connection,
    revision: &str,
    input: &SaveTreasuryTransactionInput,
) -> Result<()> {
    c.execute(
        "INSERT INTO treasury_transactions VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            revision,
            input.kind,
            input.effective_on,
            input.label,
            input.reference,
            input.notes
        ],
    )
    .map_err(db::error)?;
    for entry in &input.entries {
        c.execute(
            "INSERT INTO treasury_entries VALUES(?1,?2,?3,?4,?5)",
            params![
                revision,
                entry.line_id,
                entry.account_id,
                entry.amount,
                entry.role
            ],
        )
        .map_err(db::error)?;
    }
    Ok(())
}

pub(crate) fn normalize_link(input: &mut SaveCashLinkInput) -> Result<()> {
    input.amount = exact::normalize_decimal_input(&input.amount)?;
    Ok(())
}
fn insert_link(c: &Connection, revision: &str, input: &SaveCashLinkInput) -> Result<()> {
    c.execute(
        "INSERT INTO cash_links VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            revision,
            input.source_kind,
            input.source_id,
            input.source_revision_id,
            input.transaction_revision_id,
            input.line_id,
            input.amount,
            input.component,
            input.side,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}
fn insert_deposit(c: &Connection, revision: &str, input: &SaveDepositMovementInput) -> Result<()> {
    c.execute(
        "INSERT INTO deposit_movements VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            revision,
            input.lease_id,
            input.kind,
            input.amount,
            input.currency,
            input.effective_on,
            input.recipient_entity_id,
            input.due_id,
            input.component,
            input.notes
        ],
    )
    .map_err(db::error)?;
    Ok(())
}

fn unchanged(
    c: &Connection,
    table: &str,
    column: &str,
    object: Option<&str>,
    value: &str,
) -> Result<()> {
    if let Some(object) = object {
        let old:String=c.query_row(&format!("SELECT v.{column} FROM {table} v JOIN revisions r ON r.id=v.revision_id JOIN change_sets cs ON cs.id=r.change_set_id WHERE r.object_id=?1 ORDER BY cs.sequence DESC LIMIT 1"),[object],|r|r.get(0)).map_err(db::error)?;
        if old != value {
            return Err(AppError::new("validation","Cette identité de trésorerie conserve son actif, sa devise et sa date de suivi d’origine. Créez un nouveau compte pour un changement réel."));
        }
    }
    Ok(())
}

impl Vault {
    fn write_treasury<T>(
        &mut self,
        kind: &str,
        object: Option<&str>,
        expected: Option<&str>,
        reason: &str,
        write: impl FnOnce(&Transaction<'_>, &str, &str) -> Result<()>,
        read: impl FnOnce(&Connection, &str) -> Result<T>,
    ) -> Result<T> {
        self.ensure_active()?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        let (id, rev) = registry::begin(&tx, object, expected, kind, reason, false)?;
        write(&tx, &id, &rev)?;
        finance::validate_current(&tx)?;
        let result = read(&tx, &id)?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
}

impl TreasuryService for Vault {
    fn list_treasury_at(&self, knowledge_sequence: Option<u64>) -> Result<TreasurySnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_treasury_account(
        &mut self,
        input: SaveTreasuryAccountInput,
    ) -> Result<TreasuryAccount> {
        self.write_treasury(
            "treasury_account",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                for (column, value) in [
                    ("asset_id", input.asset_id.as_str()),
                    ("currency", input.currency.as_str()),
                    ("tracking_start", input.tracking_start.as_str()),
                ] {
                    unchanged(tx, "treasury_accounts", column, input.id.as_deref(), value)?;
                }
                insert_account(tx, rev, &input)
            },
            |tx, key| {
                snapshot(tx, None)?
                    .accounts
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le compte enregistré est introuvable."))
            },
        )
    }
    fn save_treasury_transaction(
        &mut self,
        mut input: SaveTreasuryTransactionInput,
    ) -> Result<TreasuryTransaction> {
        normalize_transaction(&mut input)?;
        self.write_treasury(
            "treasury_transaction",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| insert_transaction(tx, rev, &input),
            |tx, key| {
                snapshot(tx, None)?
                    .transactions
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("L’écriture enregistrée est introuvable."))
            },
        )
    }
    fn save_account_reconciliation(
        &mut self,
        mut input: SaveAccountReconciliationInput,
    ) -> Result<AccountReconciliation> {
        input.statement_balance = exact::normalize_decimal_input(&input.statement_balance)?;
        self.write_treasury(
            "account_reconciliation",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                unchanged(
                    tx,
                    "account_reconciliations",
                    "account_id",
                    input.id.as_deref(),
                    &input.account_id,
                )?;
                tx.execute(
                    "INSERT INTO account_reconciliations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                    params![
                        rev,
                        input.account_id,
                        input.effective_on,
                        input.statement_balance,
                        input.statement_reference,
                        input.document_id,
                        input.notes
                    ],
                )
                .map_err(db::error)?;
                Ok(())
            },
            |tx, key| {
                snapshot(tx, None)?
                    .reconciliations
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le rapprochement enregistré est introuvable."))
            },
        )
    }
    fn save_cash_link(&mut self, mut input: SaveCashLinkInput) -> Result<CashLink> {
        normalize_link(&mut input)?;
        self.write_treasury(
            "cash_link",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| insert_link(tx, rev, &input),
            |tx, key| {
                snapshot(tx, None)?
                    .cash_links
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le rapprochement de flux est introuvable."))
            },
        )
    }
    fn save_deposit_movement(
        &mut self,
        mut input: SaveDepositMovementInput,
    ) -> Result<DepositMovement> {
        input.amount = exact::normalize_decimal_input(&input.amount)?;
        self.write_treasury(
            "deposit_movement",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, rev| {
                unchanged(
                    tx,
                    "deposit_movements",
                    "lease_id",
                    input.id.as_deref(),
                    &input.lease_id,
                )?;
                insert_deposit(tx, rev, &input)
            },
            |tx, key| {
                snapshot(tx, None)?
                    .deposits
                    .into_iter()
                    .find(|r| r.id == key)
                    .ok_or_else(|| invalid("Le dépôt enregistré est introuvable."))
            },
        )
    }
    fn post_rent_payment(&mut self, mut input: PostRentPaymentInput) -> Result<RentPayment> {
        self.ensure_active()?;
        input.payment.amount = exact::normalize_decimal_input(&input.payment.amount)?;
        if input.payment.unlinked_reason.trim().is_empty() {
            input.payment.unlinked_reason =
                "Rapproché par une ligne de trésorerie locale versionnée.".into();
        }
        for allocation in &mut input.allocations {
            allocation.amount = exact::normalize_decimal_input(&allocation.amount)?;
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        expect_sequence(&tx, input.expected_sequence)?;
        let old_treasury = snapshot(&tx, None)?;
        let old_rentals = rentals::snapshot(&tx, None)?;
        if let Some(id) = &input.payment.id {
            let old = old_rentals
                .payments
                .iter()
                .find(|p| &p.id == id)
                .ok_or_else(|| {
                    AppError::new("conflict", "Le paiement à corriger n’est plus courant.")
                })?;
            if old.lease_id != input.payment.lease_id {
                return Err(AppError::new(
                    "validation",
                    "Le paiement conserve son bail d’origine.",
                ));
            }
        }
        let (change, stamp) = db::mutation(&tx, input.payment.reason.trim())?;
        let (id, revision) = registry::begin_shared(
            &tx,
            input.payment.id.as_deref(),
            input.payment.expected_revision_id.as_deref(),
            "rent_payment",
            &input.payment.reason,
            false,
            Some((&change, &stamp)),
        )?;
        rentals::insert_payment(&tx, &revision, &input.payment)?;
        for old in old_rentals
            .allocations
            .iter()
            .filter(|a| a.payment_id == id)
        {
            let (_, rev) = registry::begin_shared(
                &tx,
                Some(&old.id),
                Some(&old.revision_id),
                "rent_allocation",
                &input.payment.reason,
                true,
                Some((&change, &stamp)),
            )?;
            tx.execute(
                "INSERT INTO rent_allocations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    rev,
                    old.payment_id,
                    old.due_id,
                    old.component,
                    old.amount,
                    old.currency,
                    old.effective_on
                ],
            )
            .map_err(db::error)?;
        }
        for allocation in &input.allocations {
            let (_, rev) = registry::begin_shared(
                &tx,
                None,
                None,
                "rent_allocation",
                &input.payment.reason,
                false,
                Some((&change, &stamp)),
            )?;
            tx.execute(
                "INSERT INTO rent_allocations VALUES(?1,?2,?3,?4,?5,?6,?7)",
                params![
                    rev,
                    id,
                    allocation.due_id,
                    allocation.component,
                    allocation.amount,
                    input.payment.currency,
                    input.payment.effective_on
                ],
            )
            .map_err(db::error)?;
        }
        let old_links = source_links(&old_treasury, "rent_payment", &id);
        let transaction_revision = resolve_posting(
            &tx,
            &mut input.cash,
            &old_links,
            &input.payment.reason,
            (&change, &stamp),
        )?;
        replace_source_links(&tx, &old_links, &input.payment.reason, (&change, &stamp))?;
        create_posted_link(
            &tx,
            PostedLink {
                source_kind: "rent_payment",
                source_id: &id,
                source_revision: &revision,
                transaction_revision: &transaction_revision,
                line_id: &input.cash.line_id,
                amount: &input.payment.amount,
                component: "full",
                side: "",
            },
            &input.payment.reason,
            (&change, &stamp),
        )?;
        finance::validate_current(&tx)?;
        let result = rentals::snapshot(&tx, None)?
            .payments
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("Le paiement rapproché est introuvable."))?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn post_rental_expense(&mut self, mut input: PostRentalExpenseInput) -> Result<RentalExpense> {
        self.ensure_active()?;
        input.expense.amount = exact::normalize_decimal_input(&input.expense.amount)?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        expect_sequence(&tx, input.expected_sequence)?;
        let before = snapshot(&tx, None)?;
        unchanged(
            &tx,
            "rental_expenses",
            "asset_id",
            input.expense.id.as_deref(),
            &input.expense.asset_id,
        )?;
        let (change, stamp) = db::mutation(&tx, input.expense.reason.trim())?;
        let (id, revision) = registry::begin_shared(
            &tx,
            input.expense.id.as_deref(),
            input.expense.expected_revision_id.as_deref(),
            "rental_expense",
            &input.expense.reason,
            false,
            Some((&change, &stamp)),
        )?;
        tx.execute(
            "INSERT INTO rental_expenses VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![
                revision,
                input.expense.asset_id,
                input.expense.kind,
                input.expense.amount,
                input.expense.currency,
                input.expense.effective_on,
                input.expense.paid_on,
                input.expense.recoverable,
                input.expense.notes
            ],
        )
        .map_err(db::error)?;
        let old_links = source_links(&before, "rental_expense", &id);
        let transaction_revision = resolve_posting(
            &tx,
            &mut input.cash,
            &old_links,
            &input.expense.reason,
            (&change, &stamp),
        )?;
        replace_source_links(&tx, &old_links, &input.expense.reason, (&change, &stamp))?;
        let signed = opposite(&input.expense.amount)?;
        create_posted_link(
            &tx,
            PostedLink {
                source_kind: "rental_expense",
                source_id: &id,
                source_revision: &revision,
                transaction_revision: &transaction_revision,
                line_id: &input.cash.line_id,
                amount: &signed,
                component: "full",
                side: "",
            },
            &input.expense.reason,
            (&change, &stamp),
        )?;
        finance::validate_current(&tx)?;
        let result = rentals::snapshot(&tx, None)?
            .expenses
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("La dépense rapprochée est introuvable."))?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn post_deposit(&mut self, mut input: PostDepositInput) -> Result<DepositMovement> {
        self.ensure_active()?;
        input.movement.amount = exact::normalize_decimal_input(&input.movement.amount)?;
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        expect_sequence(&tx, input.expected_sequence)?;
        let before = snapshot(&tx, None)?;
        unchanged(
            &tx,
            "deposit_movements",
            "lease_id",
            input.movement.id.as_deref(),
            &input.movement.lease_id,
        )?;
        let (change, stamp) = db::mutation(&tx, input.movement.reason.trim())?;
        let (id, revision) = registry::begin_shared(
            &tx,
            input.movement.id.as_deref(),
            input.movement.expected_revision_id.as_deref(),
            "deposit_movement",
            &input.movement.reason,
            false,
            Some((&change, &stamp)),
        )?;
        insert_deposit(&tx, &revision, &input.movement)?;
        let old_links = source_links(&before, "deposit_movement", &id);
        if matches!(
            input.movement.kind.as_str(),
            "applied_to_due" | "adjustment"
        ) {
            if input.cash.is_some() || !old_links.is_empty() {
                return Err(AppError::new("validation","Une compensation de dépôt ne crée pas de trésorerie et ne remplace pas une réception déjà rapprochée."));
            }
        } else {
            let cash = input.cash.as_mut().ok_or_else(|| {
                AppError::new(
                    "validation",
                    "Une réception ou restitution de dépôt doit désigner sa trésorerie réelle.",
                )
            })?;
            let transaction_revision = resolve_posting(
                &tx,
                cash,
                &old_links,
                &input.movement.reason,
                (&change, &stamp),
            )?;
            replace_source_links(&tx, &old_links, &input.movement.reason, (&change, &stamp))?;
            create_posted_link(
                &tx,
                PostedLink {
                    source_kind: "deposit_movement",
                    source_id: &id,
                    source_revision: &revision,
                    transaction_revision: &transaction_revision,
                    line_id: &cash.line_id,
                    amount: &input.movement.amount,
                    component: "full",
                    side: "",
                },
                &input.movement.reason,
                (&change, &stamp),
            )?;
        }
        finance::validate_current(&tx)?;
        let result = snapshot(&tx, None)?
            .deposits
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("Le dépôt enregistré est introuvable."))?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn post_debt_movement(&mut self, mut input: PostDebtMovementInput) -> Result<DebtMovement> {
        self.ensure_active()?;
        debts::normalize_debt_movement(&mut input.movement)?;
        if input.movement.unlinked_reason.trim().is_empty() {
            input.movement.unlinked_reason =
                "Rapproché par des lignes de trésorerie locales versionnées.".into();
        }
        for link in &mut input.links {
            link.amount = exact::normalize_decimal_input(&link.amount)?;
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        expect_sequence(&tx, input.expected_sequence)?;
        let before = snapshot(&tx, None)?;
        unchanged(
            &tx,
            "debt_movements",
            "debt_id",
            input.movement.id.as_deref(),
            &input.movement.debt_id,
        )?;
        let (change, stamp) = db::mutation(&tx, input.movement.reason.trim())?;
        let (id, revision) = registry::begin_shared(
            &tx,
            input.movement.id.as_deref(),
            input.movement.expected_revision_id.as_deref(),
            "debt_movement",
            &input.movement.reason,
            false,
            Some((&change, &stamp)),
        )?;
        debts::insert_debt_movement(&tx, &revision, &input.movement)?;
        let old_links = source_links(&before, "debt_movement", &id);
        let mut cash = CashPostingInput {
            transaction: input.transaction,
            existing_transaction_revision_id: input.existing_transaction_revision_id,
            line_id: String::new(),
        };
        let transaction_revision = resolve_posting(
            &tx,
            &mut cash,
            &old_links,
            &input.movement.reason,
            (&change, &stamp),
        )?;
        replace_source_links(&tx, &old_links, &input.movement.reason, (&change, &stamp))?;
        if input.links.is_empty() {
            return Err(AppError::new(
                "validation",
                "Le mouvement financé doit relier ses composantes aux lignes de trésorerie.",
            ));
        }
        for link in &input.links {
            create_posted_link(
                &tx,
                PostedLink {
                    source_kind: "debt_movement",
                    source_id: &id,
                    source_revision: &revision,
                    transaction_revision: &transaction_revision,
                    line_id: &link.line_id,
                    amount: &link.amount,
                    component: &link.component,
                    side: &link.side,
                },
                &input.movement.reason,
                (&change, &stamp),
            )?;
        }
        finance::validate_current(&tx)?;
        let result = debts::snapshot(&tx, None)?
            .movements
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("Le mouvement financé est introuvable."))?;
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn get_treasury_report(&self, request: TreasuryReportRequest) -> Result<TreasuryReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let k = request.knowledge_sequence;
        treasury_report(
            &registry::snapshot(&tx, k)?,
            &properties::snapshot(&tx, k)?,
            &rentals::snapshot(&tx, k)?,
            &snapshot(&tx, k)?,
            &request,
        )
    }
}

fn expect_sequence(c: &Connection, expected: u64) -> Result<()> {
    if db::sequence(c)? != expected {
        return Err(AppError::new(
            "conflict",
            "Le coffre a changé. Rechargez les sources avant de poster ensemble leurs écritures.",
        ));
    }
    Ok(())
}
fn opposite(amount: &str) -> Result<String> {
    if amount == "0" {
        return Ok("0".into());
    }
    if let Some(unsigned) = amount.strip_prefix('-') {
        Ok(unsigned.into())
    } else {
        Ok(format!("-{amount}"))
    }
}
fn source_links(s: &TreasurySnapshot, kind: &str, id: &str) -> Vec<CashLink> {
    s.cash_links
        .iter()
        .filter(|l| l.source_kind == kind && l.source_id == id)
        .cloned()
        .collect()
}

fn resolve_posting(
    c: &Connection,
    cash: &mut CashPostingInput,
    old_links: &[CashLink],
    reason: &str,
    shared: (&str, &str),
) -> Result<String> {
    match (
        &mut cash.transaction,
        &cash.existing_transaction_revision_id,
    ) {
        (Some(input), None) => {
            normalize_transaction(input)?;
            let before = snapshot(c, None)?;
            if !old_links.is_empty() {
                let Some(object) = input.id.as_deref() else {
                    return Err(AppError::new("validation","La source possède déjà sa trésorerie. Corrigez cette transaction ou rapprochez une ligne existante ; une deuxième transaction ne sera pas créée."));
                };
                for link in old_links {
                    if !before
                        .transactions
                        .iter()
                        .any(|t| t.revision_id == link.transaction_revision_id && t.id == object)
                    {
                        return Err(AppError::new("validation","Les anciennes lignes rapprochées appartiennent à une autre transaction. Elles ne peuvent pas être remplacées implicitement."));
                    }
                }
            }
            let (_, revision) = registry::begin_shared(
                c,
                input.id.as_deref(),
                input.expected_revision_id.as_deref(),
                "treasury_transaction",
                reason,
                false,
                Some(shared),
            )?;
            insert_transaction(c, &revision, input)?;
            Ok(revision)
        }
        (None, Some(revision)) => {
            let before = snapshot(c, None)?;
            if !before
                .transactions
                .iter()
                .any(|t| &t.revision_id == revision)
            {
                return Err(AppError::new(
                    "conflict",
                    "La révision de trésorerie choisie n’est plus courante.",
                ));
            }
            if old_links
                .iter()
                .any(|link| &link.transaction_revision_id != revision)
            {
                return Err(AppError::new("validation","La source possède déjà sa trésorerie. Réutilisez ou corrigez cette transaction pour conserver une seule origine financière."));
            }
            Ok(revision.clone())
        }
        _ => Err(AppError::new(
            "validation",
            "Choisissez une transaction nouvelle/corrigée ou une révision existante, pas les deux.",
        )),
    }
}

fn replace_source_links(
    c: &Connection,
    old_links: &[CashLink],
    reason: &str,
    shared: (&str, &str),
) -> Result<()> {
    for old in old_links {
        let (_, revision) = registry::begin_shared(
            c,
            Some(&old.id),
            Some(&old.revision_id),
            "cash_link",
            reason,
            true,
            Some(shared),
        )?;
        c.execute("INSERT INTO cash_links SELECT ?1,source_kind,source_id,source_revision_id,transaction_revision_id,line_id,amount,component,side,notes FROM cash_links WHERE revision_id=?2",params![revision,old.revision_id]).map_err(db::error)?;
    }
    Ok(())
}
struct PostedLink<'a> {
    source_kind: &'a str,
    source_id: &'a str,
    source_revision: &'a str,
    transaction_revision: &'a str,
    line_id: &'a str,
    amount: &'a str,
    component: &'a str,
    side: &'a str,
}
fn create_posted_link(
    c: &Connection,
    link: PostedLink<'_>,
    reason: &str,
    shared: (&str, &str),
) -> Result<()> {
    let (_, revision) =
        registry::begin_shared(c, None, None, "cash_link", reason, false, Some(shared))?;
    insert_link(
        c,
        &revision,
        &SaveCashLinkInput {
            id: None,
            expected_revision_id: None,
            reason: reason.into(),
            source_kind: link.source_kind.into(),
            source_id: link.source_id.into(),
            source_revision_id: link.source_revision.into(),
            transaction_revision_id: link.transaction_revision.into(),
            line_id: link.line_id.into(),
            amount: link.amount.into(),
            component: link.component.into(),
            side: link.side.into(),
            notes: "Source et trésorerie enregistrées dans le même changement.".into(),
        },
    )
}

pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    finance::validate_record_history(c, &TABLES)?;
    for (table, column) in [
        ("treasury_accounts", "asset_id"),
        ("treasury_accounts", "currency"),
        ("treasury_accounts", "tracking_start"),
        ("account_reconciliations", "account_id"),
        ("deposit_movements", "lease_id"),
    ] {
        let bad:u64=c.query_row(&format!("SELECT count(*) FROM (SELECT r.object_id FROM {table} v JOIN revisions r ON r.id=v.revision_id GROUP BY r.object_id HAVING count(DISTINCT v.{column})<>1)"),[],|r|r.get(0)).map_err(db::error)?;
        if bad != 0 {
            return Err(invalid(
                "Une identité de trésorerie a changé d’actif, devise, début ou contrat source.",
            ));
        }
    }
    let invalid_voids:u64=c.query_row("SELECT count(*) FROM treasury_transactions v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND (EXISTS(SELECT line_id,account_id,amount,role FROM treasury_entries WHERE transaction_revision_id=r.id EXCEPT SELECT line_id,account_id,amount,role FROM treasury_entries WHERE transaction_revision_id=r.supersedes_revision_id) OR EXISTS(SELECT line_id,account_id,amount,role FROM treasury_entries WHERE transaction_revision_id=r.supersedes_revision_id EXCEPT SELECT line_id,account_id,amount,role FROM treasury_entries WHERE transaction_revision_id=r.id))",[],|r|r.get(0)).map_err(db::error)?;
    if invalid_voids != 0 {
        return Err(invalid(
            "L’annulation d’une transaction conserve ses lignes exactes.",
        ));
    }
    let invalid_sources:u64=c.query_row("SELECT count(*) FROM cash_links v JOIN revisions r ON r.id=v.revision_id JOIN change_sets lc ON lc.id=r.change_set_id JOIN revisions source ON source.id=v.source_revision_id JOIN change_sets sc ON sc.id=source.change_set_id JOIN revisions ledger ON ledger.id=v.transaction_revision_id JOIN change_sets tc ON tc.id=ledger.change_set_id WHERE source.object_id<>v.source_id OR source.is_void<>0 OR ledger.is_void<>0 OR sc.sequence>lc.sequence OR tc.sequence>lc.sequence OR NOT EXISTS(SELECT 1 FROM objects o WHERE o.id=source.object_id AND o.kind=v.source_kind)",[],|r|r.get(0)).map_err(db::error)?;
    if invalid_sources != 0 {
        return Err(invalid("Un rapprochement doit conserver des révisions source et de trésorerie connues de son changement."));
    }
    Ok(())
}
