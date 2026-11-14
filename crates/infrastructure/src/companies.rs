//! Typed company observations and treasury-linked dividend distributions.
use crate::{db, debts, files::*, finance, phase7, properties, registry, rentals, treasury, Vault};
use patrimoine_application::CompanyService;
use patrimoine_domain::*;
use rusqlite::{params, Connection, TransactionBehavior};

pub(crate) const TABLES: [(&str, &str); 2] = [
    ("company_financial_period", "company_financial_periods"),
    ("company_distribution", "company_distributions"),
];

fn links(c: &Connection, revision: &str) -> rusqlite::Result<Vec<CompanyDistributionCashLink>> {
    let mut query = c.prepare("SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=?1 ORDER BY side,transaction_revision_id,line_id")?;
    let rows = query
        .query_map([revision], |r| {
            Ok(CompanyDistributionCashLink {
                side: r.get(0)?,
                transaction_revision_id: r.get(1)?,
                line_id: r.get(2)?,
                amount: r.get(3)?,
            })
        })?
        .collect();
    rows
}

pub(crate) fn snapshot(c: &Connection, cutoff: Option<u64>) -> Result<CompanySnapshot> {
    let k = phase7::cutoff(c, cutoff)?;
    if !phase7::enabled(c)? {
        return Ok(CompanySnapshot {
            knowledge_sequence: k,
            ..Default::default()
        });
    }
    Ok(CompanySnapshot {
        financial_periods: registry::load(c,"company_financial_periods","v.company_id,v.period_start,v.period_end,v.frequency,v.currency,v.revenue,v.operating_expenses,v.operating_result,v.net_result,v.indicative_cash,v.indicative_debt,v.important_receivables,v.dividends_declared,v.indicative_global_value,v.notes","",k,|r|Ok(CompanyFinancialPeriod {id:r.get(0)?,revision_id:r.get(1)?,company_id:r.get(2)?,period_start:r.get(3)?,period_end:r.get(4)?,frequency:r.get(5)?,currency:r.get(6)?,revenue:r.get(7)?,operating_expenses:r.get(8)?,operating_result:r.get(9)?,net_result:r.get(10)?,indicative_cash:r.get(11)?,indicative_debt:r.get(12)?,important_receivables:r.get(13)?,dividends_declared:r.get(14)?,indicative_global_value:r.get(15)?,notes:r.get(16)?}))?,
        distributions: registry::load(c,"company_distributions","v.company_id,v.beneficiary_entity_id,v.kind,v.effective_on,v.amount,v.currency,v.related_declaration_id,v.unlinked_reason,v.notes","",k,|r| {let revision_id:String=r.get(1)?;Ok(CompanyDistribution {id:r.get(0)?,cash_links:links(c,&revision_id)?,revision_id,company_id:r.get(2)?,beneficiary_entity_id:r.get(3)?,kind:r.get(4)?,effective_on:r.get(5)?,amount:r.get(6)?,currency:r.get(7)?,related_declaration_id:r.get(8)?,unlinked_reason:r.get(9)?,notes:r.get(10)?})})?,
        knowledge_sequence:k,
    })
}

fn normalize_distribution(input: &mut SaveCompanyDistributionInput) -> Result<()> {
    input.amount = exact::normalize_decimal_input(&input.amount)?;
    for link in &mut input.cash_links {
        link.amount = exact::normalize_decimal_input(&link.amount)?;
    }
    Ok(())
}
fn immutable_distribution(c: &Connection, input: &SaveCompanyDistributionInput) -> Result<()> {
    for (column, value) in [
        ("company_id", input.company_id.as_str()),
        (
            "beneficiary_entity_id",
            input.beneficiary_entity_id.as_str(),
        ),
        ("kind", input.kind.as_str()),
        ("currency", input.currency.as_str()),
    ] {
        phase7::unchanged(
            c,
            "company_distributions",
            column,
            input.id.as_deref(),
            value,
        )?;
    }
    Ok(())
}
fn insert_distribution(
    c: &Connection,
    revision: &str,
    input: &SaveCompanyDistributionInput,
) -> Result<()> {
    c.execute(
        "INSERT INTO company_distributions VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            revision,
            input.company_id,
            input.beneficiary_entity_id,
            input.kind,
            input.effective_on,
            input.amount,
            input.currency,
            input.related_declaration_id,
            input.unlinked_reason,
            input.notes
        ],
    )
    .map_err(db::error)?;
    for link in &input.cash_links {
        c.execute(
            "INSERT INTO company_distribution_cash_links VALUES(?1,?2,?3,?4,?5)",
            params![
                revision,
                link.side,
                link.transaction_revision_id,
                link.line_id,
                link.amount
            ],
        )
        .map_err(db::error)?;
    }
    Ok(())
}

pub(crate) fn validate_targets(c: &Connection) -> Result<()> {
    if !phase7::enabled(c)? {
        return Ok(());
    }
    phase7::immutable_history(c, "company_financial_periods", &["company_id"])?;
    phase7::immutable_history(
        c,
        "company_distributions",
        &["company_id", "beneficiary_entity_id", "kind", "currency"],
    )
}

pub(crate) fn validate_history(c: &Connection) -> Result<()> {
    if !phase7::enabled(c)? {
        return Ok(());
    }
    validate_targets(c)?;
    finance::validate_record_history(c, &TABLES)?;
    let invalid_voids: u64 = c.query_row("SELECT count(*) FROM company_distributions v JOIN revisions r ON r.id=v.revision_id WHERE r.is_void=1 AND (EXISTS(SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=r.id EXCEPT SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=r.supersedes_revision_id) OR EXISTS(SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=r.supersedes_revision_id EXCEPT SELECT side,transaction_revision_id,line_id,amount FROM company_distribution_cash_links WHERE distribution_revision_id=r.id))",[],|r|r.get(0)).map_err(db::error)?;
    if invalid_voids != 0 {
        return Err(invalid(
            "Une annulation de distribution conserve ses lignes de trésorerie sources.",
        ));
    }
    // A revision cannot move a previously linked source to a second bank operation.
    let mut query = c.prepare("SELECT r.id,r.supersedes_revision_id FROM revisions r JOIN objects o ON o.id=r.object_id WHERE o.kind='company_distribution' AND r.supersedes_revision_id IS NOT NULL AND r.is_void=0 ORDER BY r.recorded_at,r.id").map_err(db::error)?;
    let rows = query
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db::error)?
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(db::error)?;
    for (revision, parent) in rows {
        let old = links(c, &parent)
            .map_err(db::error)?
            .into_iter()
            .map(|l| l.transaction_revision_id)
            .collect::<Vec<_>>();
        let new = links(c, &revision)
            .map_err(db::error)?
            .into_iter()
            .map(|l| l.transaction_revision_id)
            .collect::<Vec<_>>();
        phase7::preserve_cash_origins(c, &old, &new)?;
    }
    Ok(())
}

impl CompanyService for Vault {
    fn list_companies_at(&self, knowledge_sequence: Option<u64>) -> Result<CompanySnapshot> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        snapshot(&tx, knowledge_sequence)
    }
    fn save_company_financial_period(
        &mut self,
        mut input: SaveCompanyFinancialPeriodInput,
    ) -> Result<CompanyFinancialPeriod> {
        for value in [
            &mut input.revenue,
            &mut input.operating_expenses,
            &mut input.operating_result,
            &mut input.net_result,
            &mut input.indicative_cash,
            &mut input.indicative_debt,
            &mut input.important_receivables,
            &mut input.dividends_declared,
            &mut input.indicative_global_value,
        ]
        .into_iter()
        .flatten()
        {
            *value = exact::normalize_decimal_input(value)?;
        }
        let id = self.write_phase7("company_financial_period",input.id.as_deref(),input.expected_revision_id.as_deref(),&input.reason,|tx,_,revision| {
            phase7::unchanged(tx,"company_financial_periods","company_id",input.id.as_deref(),&input.company_id)?;
            tx.execute("INSERT INTO company_financial_periods VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",params![revision,input.company_id,input.period_start,input.period_end,input.frequency,input.currency,input.revenue,input.operating_expenses,input.operating_result,input.net_result,input.indicative_cash,input.indicative_debt,input.important_receivables,input.dividends_declared,input.indicative_global_value,input.notes]).map_err(db::error)?;
            Ok(())
        })?;
        self.list_companies_at(None)?
            .financial_periods
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("La période enregistrée est introuvable."))
    }
    fn save_company_distribution(
        &mut self,
        mut input: SaveCompanyDistributionInput,
    ) -> Result<CompanyDistribution> {
        normalize_distribution(&mut input)?;
        let id = self.write_phase7(
            "company_distribution",
            input.id.as_deref(),
            input.expected_revision_id.as_deref(),
            &input.reason,
            |tx, _, revision| {
                immutable_distribution(tx, &input)?;
                if let Some(parent) = &input.expected_revision_id {
                    let old = links(tx, parent).map_err(db::error)?;
                    phase7::preserve_cash_origins(
                        tx,
                        &old.iter()
                            .map(|l| l.transaction_revision_id.clone())
                            .collect::<Vec<_>>(),
                        &input
                            .cash_links
                            .iter()
                            .map(|l| l.transaction_revision_id.clone())
                            .collect::<Vec<_>>(),
                    )?;
                }
                insert_distribution(tx, revision, &input)
            },
        )?;
        self.list_companies_at(None)?
            .distributions
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| invalid("La distribution enregistrée est introuvable."))
    }
    fn post_company_distribution(
        &mut self,
        mut input: PostCompanyDistributionInput,
    ) -> Result<CompanyDistribution> {
        self.ensure_active()?;
        normalize_distribution(&mut input.distribution)?;
        if input.distribution.kind != "paid"
            || !input.distribution.cash_links.is_empty()
            || input.links.is_empty()
        {
            return Err(invalid("Le rapprochement atomique porte sur un paiement et ses lignes explicites, sans second jeu de liens dans la source."));
        }
        for link in &mut input.links {
            link.amount = exact::normalize_decimal_input(&link.amount)?;
        }
        let mut c = db::connect(&self.db_path())?;
        let tx = c
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db::error)?;
        phase7::expect_sequence(&tx, input.expected_sequence)?;
        immutable_distribution(&tx, &input.distribution)?;
        let before = snapshot(&tx, None)?;
        let old = match &input.distribution.id {
            Some(id) => Some(
                before
                    .distributions
                    .iter()
                    .find(|d| &d.id == id)
                    .ok_or_else(|| {
                        AppError::new(
                            "conflict",
                            "La distribution à corriger n’est plus courante.",
                        )
                    })?,
            ),
            None => None,
        };
        let old_revisions = old
            .into_iter()
            .flat_map(|d| {
                d.cash_links
                    .iter()
                    .map(|l| l.transaction_revision_id.clone())
            })
            .collect::<Vec<_>>();
        let (change, stamp) = db::mutation(&tx, input.distribution.reason.trim())?;
        let transaction_revision = phase7::resolve_cash(
            &tx,
            &mut input.transaction,
            &input.existing_transaction_revision_id,
            &old_revisions,
            &input.distribution.reason,
            (&change, &stamp),
        )?;
        input.distribution.cash_links = input
            .links
            .into_iter()
            .map(|l| CompanyDistributionCashLink {
                side: l.side,
                transaction_revision_id: transaction_revision.clone(),
                line_id: l.line_id,
                amount: l.amount,
            })
            .collect();
        if input.distribution.unlinked_reason.trim().is_empty() {
            input.distribution.unlinked_reason =
                "Paiement rapproché des lignes de trésorerie explicitement suivies.".into();
        }
        let (id, revision) = registry::begin_shared(
            &tx,
            input.distribution.id.as_deref(),
            input.distribution.expected_revision_id.as_deref(),
            "company_distribution",
            &input.distribution.reason,
            false,
            Some((&change, &stamp)),
        )?;
        insert_distribution(&tx, &revision, &input.distribution)?;
        finance::validate_current(&tx)?;
        let state = snapshot(&tx, None)?;
        let result = state
            .distributions
            .iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or_else(|| invalid("La distribution rapprochée est introuvable."))?;
        let (payer, beneficiary) = distribution_cash_coverage(&state, &result)?;
        if result.cash_links.iter().any(|l| l.side == "payer") && !payer
            || result.cash_links.iter().any(|l| l.side == "beneficiary") && !beneficiary
        {
            return Err(invalid("Chaque côté rapproché atomiquement couvre son montant entier ; laissez explicite la contrepartie non suivie."));
        }
        tx.commit().map_err(db::error)?;
        Ok(result)
    }
    fn get_company_report(&self, request: CompanyReportRequest) -> Result<CompanyReport> {
        self.ensure_active()?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let k = request.knowledge_sequence;
        company_report(
            &registry::snapshot(&tx, k)?,
            &properties::snapshot(&tx, k)?,
            &rentals::snapshot(&tx, k)?,
            &treasury::snapshot(&tx, k)?,
            &debts::snapshot(&tx, k)?,
            &snapshot(&tx, k)?,
            &request,
        )
    }
}
