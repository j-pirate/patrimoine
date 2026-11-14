//! Coherent dashboards computed from one immutable set of historical sources.
use crate::{
    companies, db, debts, files::*, finance, metals, properties, registry, rentals, treasury, Vault,
};
use patrimoine_application::DashboardService;
use patrimoine_domain::*;
use rusqlite::Connection;
use serde::Serialize;
use std::{collections::BTreeSet, fs, io::Write, path::Path};

// Explicit work budget: never omit family members silently from a comparison.
const MAX_MEMBERS: usize = 200;

struct Sources {
    registry: RegistrySnapshot,
    properties: PropertySnapshot,
    rentals: RentalSnapshot,
    treasury: TreasurySnapshot,
    debts: DebtSnapshot,
    finance: FinanceSnapshot,
    metals: MetalSnapshot,
    companies: CompanySnapshot,
}

fn validation(message: &str) -> AppError {
    AppError::new("validation", message)
}

fn validate_scope(c: &Connection, sources: &Sources, request: &DashboardRequest) -> Result<()> {
    if request.context != "family" {
        let kind = if request.context == "company" {
            "company"
        } else {
            "person"
        };
        if !sources
            .registry
            .entities
            .iter()
            .any(|e| e.kind == kind && Some(e.id.as_str()) == request.entity_id.as_deref())
        {
            return Err(validation(
                "L’entité du tableau de bord est inconnue à ces connaissances.",
            ));
        }
    }
    if let Some(id) = &request.filters.owner_entity_id {
        if !sources
            .registry
            .entities
            .iter()
            .any(|entity| &entity.id == id)
        {
            return Err(validation(
                "Le propriétaire filtré est inconnu à ces connaissances.",
            ));
        }
    }
    for (table, value, message) in [
        (
            "countries",
            request.filters.country.as_ref(),
            "Le pays filtré est inconnu.",
        ),
        (
            "currencies",
            request.filters.currency.as_ref(),
            "La devise filtrée est inconnue.",
        ),
    ] {
        if let Some(value) = value {
            let present: bool = c
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE code=?1)"),
                    [value],
                    |row| row.get(0),
                )
                .map_err(db::error)?;
            if !present {
                return Err(validation(message));
            }
        }
    }
    Ok(())
}

impl Sources {
    fn financial(
        &self,
        context: &str,
        entity: Option<&str>,
        date: &str,
    ) -> Result<FinancialReport> {
        financial_report_with_operations(
            &self.registry,
            &self.properties,
            &self.rentals,
            &self.treasury,
            &self.debts,
            &self.finance,
            &self.metals,
            &self.companies,
            &FinancialReportRequest {
                context: context.into(),
                person_id: entity.map(str::to_owned),
                date: date.into(),
                knowledge_sequence: Some(self.registry.knowledge_sequence),
            },
        )
    }

    fn rental(&self, request: &DashboardRequest) -> Result<RentalReport> {
        financial_rental_report(
            &self.registry,
            &self.properties,
            &self.rentals,
            &self.treasury,
            &self.debts,
            &self.finance,
            &RentalReportRequest {
                country: request.filters.country.clone(),
                context: request.context.clone(),
                person_id: request.entity_id.clone(),
                asset_id: None,
                from_date: request.from_date.clone(),
                to_date: request.date.clone(),
                date: request.date.clone(),
                knowledge_sequence: request.knowledge_sequence,
            },
        )
    }

    fn variation(&self, request: &DashboardRequest) -> Result<VariationReport> {
        // Keep precisely the historical bridge's existing scope: calculated
        // metal stocks must never fall back to their former generic valuation.
        let mut registry = self.registry.clone();
        let metal_assets: BTreeSet<_> = self
            .metals
            .lots
            .iter()
            .map(|lot| lot.asset_id.as_str())
            .collect();
        registry.valuations.retain(|value| {
            value.target_kind != "asset" || !metal_assets.contains(value.target_id.as_str())
        });
        let mut report = explain_variation(
            &registry,
            &VariationRequest {
                context: request.context.clone(),
                person_id: request.entity_id.clone(),
                from_date: request.from_date.clone(),
                to_date: request.date.clone(),
                knowledge_sequence: request.knowledge_sequence,
            },
        )?;
        if !metal_assets.is_empty() {
            report.status = "partial".into();
            report.warnings.push(ConsolidationWarning {
                code: "metal_stocks_excluded_from_bridge".into(), object_id: "scope".into(),
                message: "Ce pont historique porte sur les estimations du registre. Les stocks métal calculés sont exclus ; leur ancienne estimation générique n’a pas été réutilisée et aucune performance globale n’est déduite.".into(),
            });
        }
        Ok(report)
    }
}

#[derive(Serialize)]
struct FilteredReport<'a> {
    report_kind: &'static str,
    format_version: u32,
    schema_version: u32,
    app_version: &'static str,
    generated_at: String,
    purpose: &'static str,
    excluded_sections: Vec<&'static str>,
    selection_note: Option<&'static str>,
    report: &'a DashboardReport,
}

impl DashboardService for Vault {
    fn get_dashboard(&self, mut request: DashboardRequest) -> Result<DashboardReport> {
        self.ensure_active()?;
        validate_dashboard_request(&request)?;
        let dates = dashboard_dates(&request)?;
        let mut c = db::readonly(&self.db_path())?;
        let tx = c.transaction().map_err(db::error)?;
        let latest = db::sequence(&tx)?;
        let knowledge = request.knowledge_sequence.unwrap_or(latest);
        if knowledge > latest {
            return Err(validation(
                "La borne de connaissance demandée n’existe pas.",
            ));
        }
        request.knowledge_sequence = Some(knowledge);
        let k = Some(knowledge);
        let sources = Sources {
            registry: registry::snapshot(&tx, k)?,
            properties: properties::snapshot(&tx, k)?,
            rentals: rentals::snapshot(&tx, k)?,
            treasury: treasury::snapshot(&tx, k)?,
            debts: debts::snapshot(&tx, k)?,
            finance: finance::snapshot(&tx, k)?,
            metals: metals::snapshot(&tx, k)?,
            companies: companies::snapshot(&tx, k)?,
        };
        validate_scope(&tx, &sources, &request)?;
        // All source rows are now owned and pinned. Calculations keep no SQLite
        // read transaction open while the graphical report is assembled.
        tx.commit().map_err(db::error)?;
        // A stock can be observed for a single date. The existing rental
        // engine requires a nonempty [from, to) interval; no interval means
        // no rental report, rather than an invented zero or failed stock view.
        let rental = if request.from_date < request.date {
            Some(sources.rental(&request)?)
        } else {
            None
        };
        let mut members = Vec::new();
        let mut history = Vec::new();
        let (financial, company, variation) = if request.context == "company" {
            let mut company = company_report(
                &sources.registry,
                &sources.properties,
                &sources.rentals,
                &sources.treasury,
                &sources.debts,
                &sources.companies,
                &CompanyReportRequest {
                    company_id: request.entity_id.clone().expect("validated company"),
                    date: request.date.clone(),
                    frequency: None,
                    knowledge_sequence: k,
                },
            )?;
            // Observations remain complete source periods, selected by their
            // closing date; no prorating or cross-frequency aggregation.
            // Accounts, rights and debts above remain stocks observed at D.
            company.financial_periods.retain(|analysis| {
                analysis.period.period_end > request.from_date
                    && analysis.period.period_end <= request.date
            });
            company.distributions.retain(|status| {
                status.distribution.effective_on >= request.from_date
                    && status.distribution.effective_on <= request.date
            });
            (None, Some(company), None)
        } else {
            let financial = sources.financial(
                &request.context,
                request.entity_id.as_deref(),
                &request.date,
            )?;
            let selected: BTreeSet<_> = if request.context == "family" {
                sources
                    .registry
                    .memberships
                    .iter()
                    .filter(|m| applies(&m.valid_from, m.valid_to.as_deref(), &request.date))
                    .map(|m| m.person_id.clone())
                    .collect()
            } else {
                BTreeSet::new()
            };
            if selected.len() > MAX_MEMBERS {
                return Err(AppError::new("capacity", "La comparaison du tableau de bord est limitée à 200 membres par calcul ; aucun membre n’a été tronqué."));
            }
            for entity in selected {
                members.push((
                    entity.clone(),
                    sources.financial("person", Some(&entity), &request.date)?,
                ));
            }
            for date in dates {
                history.push(sources.financial(
                    &request.context,
                    request.entity_id.as_deref(),
                    &date,
                )?);
            }
            (Some(financial), None, Some(sources.variation(&request)?))
        };
        build_dashboard(
            DashboardSources {
                registry: &sources.registry,
                properties: &sources.properties,
                treasury: &sources.treasury,
                metals: &sources.metals,
            },
            request,
            financial,
            company,
            rental,
            members,
            history,
            variation,
        )
    }

    fn export_dashboard_report(&mut self, request: DashboardRequest, target: &Path) -> Result<()> {
        self.ensure_active()?;
        if !target
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
        {
            return Err(validation(
                "Le rapport filtré doit avoir l’extension .json.",
            ));
        }
        let parent = destination(target, &self.root)?;
        let temporary = parent.join(format!(".patrimoine-rapport-{}.json", id()));
        let result = (|| {
            let mut report = self.get_dashboard(request)?;
            // The interactive dashboard also exposes whole-scope source
            // reports for explanations. A filtered export must not smuggle
            // their unfiltered positions or flows into the selected report.
            let excluded_sections = if report.filters_applied {
                report.financial = None;
                report.rental = None;
                report.variation = None;
                vec!["financial", "rental", "variation"]
            } else {
                Vec::new()
            };
            let envelope = FilteredReport {
                report_kind: "patrimoine_filtered_report", format_version: 1,
                schema_version: db::SCHEMA, app_version: env!("CARGO_PKG_VERSION"),
                generated_at: now(),
                purpose: "Rapport filtré de consultation ; ce fichier n’est pas une sauvegarde de récupération.",
                selection_note: report.filters_applied.then_some("Les détails non filtrés du bilan source, des locations et du pont ne sont pas exportés quand la sélection de positions est filtrée."),
                excluded_sections,
                report: &report,
            };
            let mut file = new_file(&temporary)?;
            serde_json::to_writer_pretty(&mut file, &envelope).map_err(|error| {
                AppError::new(
                    "io",
                    format!("Le rapport JSON ne peut pas être écrit : {error}"),
                )
            })?;
            file.write_all(b"\n").map_err(io)?;
            file.sync_all().map_err(io)?;
            publish_file(&temporary, target)
        })();
        let _ = fs::remove_file(&temporary);
        result
    }
}
