#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use patrimoine_application::{
    CompanyService, DashboardService, DebtService, DocumentService, FinancialService, MetalService,
    PortabilityService, PropertyService, RegistryService, RentalService, TreasuryService,
    VaultService,
};
use patrimoine_domain::{
    AppError, Asset, BackupSummary, CompanyPolicy, ConsolidationReport, ConsolidationRequest,
    DocumentSummary, Entity, ExchangeRate, FxQuery, FxResult, Membership, Ownership, Participation,
    ProfilePeriod, RegistryHistory, RegistrySnapshot, RestoreInspection, SaveAssetInput,
    SaveCompanyPolicyInput, SaveEntityInput, SaveExchangeRateInput, SaveMembershipInput,
    SaveOwnershipInput, SaveParticipationInput, SaveProfilePeriodInput, SaveValuationInput,
    Settings, Valuation, VariationReport, VariationRequest, VaultStatus, VoidRecordInput,
};
use patrimoine_infrastructure::Vault;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use tauri_plugin_fs::FsExt;

struct DesktopState(Arc<Mutex<Vault>>);

// Les dialogues accordent une portée sur le fichier choisi, sans donner au JS
// une API de lecture/écriture générique. Le backend vérifie cette portée.
fn require_selected_file(app: &tauri::AppHandle, path: &str) -> Result<(), AppError> {
    let path = std::path::Path::new(path);
    if !path.is_absolute() || !app.fs_scope().is_allowed(path) {
        return Err(AppError::new(
            "FILE_SCOPE",
            "Sélectionnez ce fichier avec le dialogue de l’application.",
        ));
    }
    Ok(())
}

fn data_directory() -> Result<PathBuf, String> {
    if let Some(root) = std::env::var_os("PATRIMOINE_DATA_DIR") {
        let root = PathBuf::from(root);
        if !root.is_absolute() {
            return Err("PATRIMOINE_DATA_DIR doit être un chemin absolu.".into());
        }
        return Ok(root);
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .ok_or("Le répertoire personnel Linux est introuvable.")?;
    Ok(base.join("patrimoine"))
}

async fn with_vault<T: Send + 'static>(
    state: State<'_, DesktopState>,
    action: impl FnOnce(&mut Vault) -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    let vault = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut vault = vault.lock().map_err(|_| AppError {
            code: "VAULT_LOCK".into(),
            message:
                "Le coffre est indisponible. Redémarrez l’application sans supprimer ses données."
                    .into(),
        })?;
        action(&mut vault)
    })
    .await
    .map_err(|_| AppError {
        code: "TASK_FAILED".into(),
        message:
            "L’opération locale a été interrompue. Les données conservées restent dans le coffre."
                .into(),
    })?
}

#[tauri::command]
async fn get_status(state: State<'_, DesktopState>) -> Result<VaultStatus, AppError> {
    with_vault(state, |vault| vault.status()).await
}

#[tauri::command]
async fn get_settings(state: State<'_, DesktopState>) -> Result<Settings, AppError> {
    with_vault(state, |vault| Ok(vault.status()?.settings)).await
}

#[tauri::command]
async fn get_dashboard(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::DashboardRequest,
) -> Result<patrimoine_domain::DashboardReport, AppError> {
    with_vault(state, move |vault| vault.get_dashboard(request)).await
}

#[tauri::command]
async fn export_dashboard_report(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    request: patrimoine_domain::DashboardRequest,
    destination_path: String,
) -> Result<(), AppError> {
    require_selected_file(&app, &destination_path)?;
    with_vault(state, move |vault| {
        vault.export_dashboard_report(request, &PathBuf::from(destination_path))
    })
    .await
}

#[tauri::command]
async fn save_settings(
    state: State<'_, DesktopState>,
    settings: Settings,
) -> Result<Settings, AppError> {
    with_vault(state, move |vault| vault.save_settings(settings)).await
}

#[tauri::command]
async fn list_documents(state: State<'_, DesktopState>) -> Result<Vec<DocumentSummary>, AppError> {
    with_vault(state, |vault| vault.list_documents()).await
}

#[tauri::command]
async fn import_document(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    source_path: String,
    description: Option<String>,
) -> Result<DocumentSummary, AppError> {
    require_selected_file(&app, &source_path)?;
    with_vault(state, move |vault| {
        vault.import_document(
            &PathBuf::from(source_path),
            description.as_deref().unwrap_or(""),
        )
    })
    .await
}

#[tauri::command]
async fn export_document(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    document_id: String,
    destination_path: String,
) -> Result<(), AppError> {
    require_selected_file(&app, &destination_path)?;
    with_vault(state, move |vault| {
        vault.export_document(&document_id, &PathBuf::from(destination_path))
    })
    .await
}

#[tauri::command]
async fn create_backup(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    destination_path: String,
) -> Result<BackupSummary, AppError> {
    require_selected_file(&app, &destination_path)?;
    with_vault(state, move |vault| {
        vault.create_backup(&PathBuf::from(destination_path))
    })
    .await
}

#[tauri::command]
async fn inspect_restore(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    archive_path: String,
) -> Result<RestoreInspection, AppError> {
    require_selected_file(&app, &archive_path)?;
    with_vault(state, move |vault| {
        vault.inspect_restore(&PathBuf::from(archive_path))
    })
    .await
}

#[tauri::command]
async fn activate_restore(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    token: String,
    confirmed: bool,
    safety_backup_path: Option<String>,
) -> Result<VaultStatus, AppError> {
    if let Some(path) = &safety_backup_path {
        require_selected_file(&app, path)?;
    }
    with_vault(state, move |vault| {
        vault.activate_restore(
            &token,
            confirmed,
            safety_backup_path.as_deref().map(std::path::Path::new),
        )
    })
    .await
}

#[tauri::command]
async fn list_registry(state: State<'_, DesktopState>) -> Result<RegistrySnapshot, AppError> {
    with_vault(state, |vault| vault.list_registry()).await
}

#[tauri::command]
async fn save_entity(
    state: State<'_, DesktopState>,
    input: SaveEntityInput,
) -> Result<Entity, AppError> {
    with_vault(state, move |vault| vault.save_entity(input)).await
}

#[tauri::command]
async fn save_membership(
    state: State<'_, DesktopState>,
    input: SaveMembershipInput,
) -> Result<Membership, AppError> {
    with_vault(state, move |vault| vault.save_membership(input)).await
}

#[tauri::command]
async fn save_participation(
    state: State<'_, DesktopState>,
    input: SaveParticipationInput,
) -> Result<Participation, AppError> {
    with_vault(state, move |vault| vault.save_participation(input)).await
}

#[tauri::command]
async fn save_asset(
    state: State<'_, DesktopState>,
    input: SaveAssetInput,
) -> Result<Asset, AppError> {
    with_vault(state, move |vault| vault.save_asset(input)).await
}

#[tauri::command]
async fn save_ownership(
    state: State<'_, DesktopState>,
    input: SaveOwnershipInput,
) -> Result<Ownership, AppError> {
    with_vault(state, move |vault| vault.save_ownership(input)).await
}

#[tauri::command]
async fn save_valuation(
    state: State<'_, DesktopState>,
    input: SaveValuationInput,
) -> Result<Valuation, AppError> {
    with_vault(state, move |vault| vault.save_valuation(input)).await
}

#[tauri::command]
async fn save_company_policy(
    state: State<'_, DesktopState>,
    input: SaveCompanyPolicyInput,
) -> Result<CompanyPolicy, AppError> {
    with_vault(state, move |vault| vault.save_company_policy(input)).await
}

#[tauri::command]
async fn void_registry_record(
    state: State<'_, DesktopState>,
    input: VoidRecordInput,
) -> Result<(), AppError> {
    with_vault(state, move |vault| vault.void_registry_record(input)).await
}

#[tauri::command]
async fn get_consolidation(
    state: State<'_, DesktopState>,
    request: ConsolidationRequest,
) -> Result<ConsolidationReport, AppError> {
    with_vault(state, move |vault| vault.get_consolidation(request)).await
}

#[tauri::command]
async fn list_registry_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<RegistrySnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_registry_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn list_registry_history(
    state: State<'_, DesktopState>,
) -> Result<RegistryHistory, AppError> {
    with_vault(state, |vault| vault.list_registry_history()).await
}

#[tauri::command]
async fn save_exchange_rate(
    state: State<'_, DesktopState>,
    input: SaveExchangeRateInput,
) -> Result<ExchangeRate, AppError> {
    with_vault(state, move |vault| vault.save_exchange_rate(input)).await
}

#[tauri::command]
async fn save_profile_period(
    state: State<'_, DesktopState>,
    input: SaveProfilePeriodInput,
) -> Result<ProfilePeriod, AppError> {
    with_vault(state, move |vault| vault.save_profile_period(input)).await
}

#[tauri::command]
async fn convert_currency(
    state: State<'_, DesktopState>,
    query: FxQuery,
) -> Result<FxResult, AppError> {
    with_vault(state, move |vault| vault.convert_currency(query)).await
}

#[tauri::command]
async fn explain_variation(
    state: State<'_, DesktopState>,
    request: VariationRequest,
) -> Result<VariationReport, AppError> {
    with_vault(state, move |vault| vault.explain_variation(request)).await
}

#[tauri::command]
async fn list_properties_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::PropertySnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_properties_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn save_property(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SavePropertyInput,
) -> Result<patrimoine_domain::Property, AppError> {
    with_vault(state, move |vault| vault.save_property(input)).await
}

#[tauri::command]
async fn save_rental_unit(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentalUnitInput,
) -> Result<patrimoine_domain::RentalUnit, AppError> {
    with_vault(state, move |vault| vault.save_rental_unit(input)).await
}

#[tauri::command]
async fn save_property_event(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SavePropertyEventInput,
) -> Result<patrimoine_domain::PropertyEvent, AppError> {
    with_vault(state, move |vault| vault.save_property_event(input)).await
}

#[tauri::command]
async fn dispose_property(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::DisposePropertyInput,
) -> Result<patrimoine_domain::PropertySnapshot, AppError> {
    with_vault(state, move |vault| vault.dispose_property(input)).await
}

#[tauri::command]
async fn get_property_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::PropertyReportRequest,
) -> Result<patrimoine_domain::PropertyReport, AppError> {
    with_vault(state, move |vault| vault.get_property_report(request)).await
}

#[tauri::command]
async fn list_rentals_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::RentalSnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_rentals_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn save_lease(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveLeaseInput,
) -> Result<patrimoine_domain::Lease, AppError> {
    with_vault(state, move |vault| vault.save_lease(input)).await
}

#[tauri::command]
async fn save_lease_terms(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveLeaseTermsInput,
) -> Result<patrimoine_domain::LeaseTerms, AppError> {
    with_vault(state, move |vault| vault.save_lease_terms(input)).await
}

#[tauri::command]
async fn save_rent_due(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentDueInput,
) -> Result<patrimoine_domain::RentDue, AppError> {
    with_vault(state, move |vault| vault.save_rent_due(input)).await
}

#[tauri::command]
async fn save_rent_payment(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentPaymentInput,
) -> Result<patrimoine_domain::RentPayment, AppError> {
    with_vault(state, move |vault| vault.save_rent_payment(input)).await
}

#[tauri::command]
async fn save_rent_allocation(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentAllocationInput,
) -> Result<patrimoine_domain::RentAllocation, AppError> {
    with_vault(state, move |vault| vault.save_rent_allocation(input)).await
}

#[tauri::command]
async fn save_rental_expense(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentalExpenseInput,
) -> Result<patrimoine_domain::RentalExpense, AppError> {
    with_vault(state, move |vault| vault.save_rental_expense(input)).await
}

#[tauri::command]
async fn generate_rent_dues(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::GenerateRentDuesInput,
) -> Result<patrimoine_domain::RentGenerationResult, AppError> {
    with_vault(state, move |vault| vault.generate_rent_dues(input)).await
}

#[tauri::command]
async fn get_rental_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::RentalReportRequest,
) -> Result<patrimoine_domain::RentalReport, AppError> {
    with_vault(state, move |vault| vault.get_rental_report(request)).await
}

#[tauri::command]
async fn refund_rent_payment(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::RefundRentPaymentInput,
) -> Result<patrimoine_domain::RentPayment, AppError> {
    with_vault(state, move |vault| vault.refund_rent_payment(input)).await
}

#[tauri::command]
async fn list_treasury_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::TreasurySnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_treasury_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn list_debts_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::DebtSnapshot, AppError> {
    with_vault(state, move |vault| vault.list_debts_at(knowledge_sequence)).await
}

#[tauri::command]
async fn list_finance_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::FinanceSnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_finance_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn save_treasury_account(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveTreasuryAccountInput,
) -> Result<patrimoine_domain::TreasuryAccount, AppError> {
    with_vault(state, move |vault| vault.save_treasury_account(input)).await
}

#[tauri::command]
async fn save_treasury_transaction(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveTreasuryTransactionInput,
) -> Result<patrimoine_domain::TreasuryTransaction, AppError> {
    with_vault(state, move |vault| vault.save_treasury_transaction(input)).await
}

#[tauri::command]
async fn save_account_reconciliation(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveAccountReconciliationInput,
) -> Result<patrimoine_domain::AccountReconciliation, AppError> {
    with_vault(state, move |vault| vault.save_account_reconciliation(input)).await
}

#[tauri::command]
async fn save_cash_link(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveCashLinkInput,
) -> Result<patrimoine_domain::CashLink, AppError> {
    with_vault(state, move |vault| vault.save_cash_link(input)).await
}

#[tauri::command]
async fn save_deposit_movement(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDepositMovementInput,
) -> Result<patrimoine_domain::DepositMovement, AppError> {
    with_vault(state, move |vault| vault.save_deposit_movement(input)).await
}

#[tauri::command]
async fn post_rent_payment(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::PostRentPaymentInput,
) -> Result<patrimoine_domain::RentPayment, AppError> {
    with_vault(state, move |vault| vault.post_rent_payment(input)).await
}

#[tauri::command]
async fn post_rental_expense(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::PostRentalExpenseInput,
) -> Result<patrimoine_domain::RentalExpense, AppError> {
    with_vault(state, move |vault| vault.post_rental_expense(input)).await
}

#[tauri::command]
async fn post_deposit(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::PostDepositInput,
) -> Result<patrimoine_domain::DepositMovement, AppError> {
    with_vault(state, move |vault| vault.post_deposit(input)).await
}

#[tauri::command]
async fn post_debt_movement(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::PostDebtMovementInput,
) -> Result<patrimoine_domain::DebtMovement, AppError> {
    with_vault(state, move |vault| vault.post_debt_movement(input)).await
}

#[tauri::command]
async fn get_treasury_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::TreasuryReportRequest,
) -> Result<patrimoine_domain::TreasuryReport, AppError> {
    with_vault(state, move |vault| vault.get_treasury_report(request)).await
}

#[tauri::command]
async fn save_debt_contract(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtContractInput,
) -> Result<patrimoine_domain::DebtContract, AppError> {
    with_vault(state, move |vault| vault.save_debt_contract(input)).await
}

#[tauri::command]
async fn save_debt_terms(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtTermsInput,
) -> Result<patrimoine_domain::DebtTerms, AppError> {
    with_vault(state, move |vault| vault.save_debt_terms(input)).await
}

#[tauri::command]
async fn save_debt_movement(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtMovementInput,
) -> Result<patrimoine_domain::DebtMovement, AppError> {
    with_vault(state, move |vault| vault.save_debt_movement(input)).await
}

#[tauri::command]
async fn save_debt_receivable_adjustment(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtReceivableAdjustmentInput,
) -> Result<patrimoine_domain::DebtReceivableAdjustment, AppError> {
    with_vault(state, move |vault| {
        vault.save_debt_receivable_adjustment(input)
    })
    .await
}

#[tauri::command]
async fn save_debt_schedule(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtScheduleInput,
) -> Result<patrimoine_domain::DebtSchedule, AppError> {
    with_vault(state, move |vault| vault.save_debt_schedule(input)).await
}

#[tauri::command]
async fn save_debt_payment_allocation(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtPaymentAllocationInput,
) -> Result<patrimoine_domain::DebtPaymentAllocation, AppError> {
    with_vault(state, move |vault| {
        vault.save_debt_payment_allocation(input)
    })
    .await
}

#[tauri::command]
async fn save_debt_asset_link(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtAssetLinkInput,
) -> Result<patrimoine_domain::DebtAssetLink, AppError> {
    with_vault(state, move |vault| vault.save_debt_asset_link(input)).await
}

#[tauri::command]
async fn save_debt_guarantee(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtGuaranteeInput,
) -> Result<patrimoine_domain::DebtGuarantee, AppError> {
    with_vault(state, move |vault| vault.save_debt_guarantee(input)).await
}

#[tauri::command]
async fn save_debt_coverage(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDebtCoverageInput,
) -> Result<patrimoine_domain::DebtCoverage, AppError> {
    with_vault(state, move |vault| vault.save_debt_coverage(input)).await
}

#[tauri::command]
async fn get_debt_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::DebtReportRequest,
) -> Result<patrimoine_domain::DebtReport, AppError> {
    with_vault(state, move |vault| vault.get_debt_report(request)).await
}

#[tauri::command]
async fn save_finance_coverage(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveFinanceCoverageInput,
) -> Result<patrimoine_domain::FinanceCoverage, AppError> {
    with_vault(state, move |vault| vault.save_finance_coverage(input)).await
}

#[tauri::command]
async fn save_rent_claim(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentClaimInput,
) -> Result<patrimoine_domain::RentClaim, AppError> {
    with_vault(state, move |vault| vault.save_rent_claim(input)).await
}

#[tauri::command]
async fn save_rent_claim_adjustment(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveRentClaimAdjustmentInput,
) -> Result<patrimoine_domain::RentClaimAdjustment, AppError> {
    with_vault(state, move |vault| vault.save_rent_claim_adjustment(input)).await
}

#[tauri::command]
async fn get_financial_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::FinancialReportRequest,
) -> Result<patrimoine_domain::FinancialReport, AppError> {
    with_vault(state, move |vault| vault.get_financial_report(request)).await
}

#[tauri::command]
async fn list_metals_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::MetalSnapshot, AppError> {
    with_vault(state, move |vault| vault.list_metals_at(knowledge_sequence)).await
}

#[tauri::command]
async fn list_companies_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::CompanySnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_companies_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn save_metal_lot(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveMetalLotInput,
) -> Result<patrimoine_domain::MetalLot, AppError> {
    with_vault(state, move |vault| vault.save_metal_lot(input)).await
}

#[tauri::command]
async fn save_metal_movement(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveMetalMovementInput,
) -> Result<patrimoine_domain::MetalMovement, AppError> {
    with_vault(state, move |vault| vault.save_metal_movement(input)).await
}

#[tauri::command]
async fn save_metal_quote(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveMetalQuoteInput,
) -> Result<patrimoine_domain::MetalQuote, AppError> {
    with_vault(state, move |vault| vault.save_metal_quote(input)).await
}

#[tauri::command]
async fn save_metal_premium(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveMetalPremiumInput,
) -> Result<patrimoine_domain::MetalPremium, AppError> {
    with_vault(state, move |vault| vault.save_metal_premium(input)).await
}

#[tauri::command]
async fn save_metal_cash_link(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveMetalCashLinkInput,
) -> Result<patrimoine_domain::MetalCashLink, AppError> {
    with_vault(state, move |vault| vault.save_metal_cash_link(input)).await
}

#[tauri::command]
async fn post_metal_movement(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::PostMetalMovementInput,
) -> Result<patrimoine_domain::MetalMovement, AppError> {
    with_vault(state, move |vault| vault.post_metal_movement(input)).await
}

#[tauri::command]
async fn transfer_metal_ownership(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::TransferMetalOwnershipInput,
) -> Result<patrimoine_domain::RegistrySnapshot, AppError> {
    with_vault(state, move |vault| vault.transfer_metal_ownership(input)).await
}

#[tauri::command]
async fn get_metal_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::MetalReportRequest,
) -> Result<patrimoine_domain::MetalReport, AppError> {
    with_vault(state, move |vault| vault.get_metal_report(request)).await
}

#[tauri::command]
async fn save_company_financial_period(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveCompanyFinancialPeriodInput,
) -> Result<patrimoine_domain::CompanyFinancialPeriod, AppError> {
    with_vault(state, move |vault| {
        vault.save_company_financial_period(input)
    })
    .await
}

#[tauri::command]
async fn save_company_distribution(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveCompanyDistributionInput,
) -> Result<patrimoine_domain::CompanyDistribution, AppError> {
    with_vault(state, move |vault| vault.save_company_distribution(input)).await
}

#[tauri::command]
async fn post_company_distribution(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::PostCompanyDistributionInput,
) -> Result<patrimoine_domain::CompanyDistribution, AppError> {
    with_vault(state, move |vault| vault.post_company_distribution(input)).await
}

#[tauri::command]
async fn get_company_report(
    state: State<'_, DesktopState>,
    request: patrimoine_domain::CompanyReportRequest,
) -> Result<patrimoine_domain::CompanyReport, AppError> {
    with_vault(state, move |vault| vault.get_company_report(request)).await
}

#[tauri::command]
async fn list_document_library_at(
    state: State<'_, DesktopState>,
    knowledge_sequence: Option<u64>,
) -> Result<patrimoine_domain::DocumentLibrarySnapshot, AppError> {
    with_vault(state, move |vault| {
        vault.list_document_library_at(knowledge_sequence)
    })
    .await
}

#[tauri::command]
async fn save_document_metadata(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDocumentMetadataInput,
) -> Result<patrimoine_domain::DocumentRecord, AppError> {
    with_vault(state, move |vault| vault.save_document_metadata(input)).await
}

#[tauri::command]
async fn save_document_link(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::SaveDocumentLinkInput,
) -> Result<patrimoine_domain::DocumentLink, AppError> {
    with_vault(state, move |vault| vault.save_document_link(input)).await
}

#[tauri::command]
async fn remove_document_link(
    state: State<'_, DesktopState>,
    input: patrimoine_domain::VoidRecordInput,
) -> Result<(), AppError> {
    with_vault(state, move |vault| vault.remove_document_link(input)).await
}

#[tauri::command]
async fn replace_document(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    input: patrimoine_domain::ReplaceDocumentInput,
    source_path: String,
) -> Result<patrimoine_domain::DocumentRecord, AppError> {
    require_selected_file(&app, &source_path)?;
    with_vault(state, move |vault| {
        vault.replace_document(input, &PathBuf::from(source_path))
    })
    .await
}
#[tauri::command]
async fn export_document_revision(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    document_id: String,
    revision_id: String,
    destination_path: String,
) -> Result<(), AppError> {
    require_selected_file(&app, &destination_path)?;
    with_vault(state, move |vault| {
        vault.export_document_revision(&document_id, &revision_id, &PathBuf::from(destination_path))
    })
    .await
}
#[tauri::command]
async fn open_document_revision(
    state: State<'_, DesktopState>,
    document_id: String,
    revision_id: String,
) -> Result<(), AppError> {
    let prepared = with_vault(state, move |vault| {
        vault.prepare_document_open(&document_id, &revision_id)
    })
    .await?;
    // The verified private copy is handed to the desktop without a shell.
    let mut child = std::process::Command::new("xdg-open").arg(&prepared.path)
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map_err(|_| AppError::new("document_open", "Le système Linux n’a pas pu recevoir la demande d’ouverture. Exportez une copie du document pour l’ouvrir dans votre application."))?;
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}

#[tauri::command]
async fn export_csv_archive(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    destination_path: String,
) -> Result<patrimoine_domain::ExportSummary, AppError> {
    require_selected_file(&app, &destination_path)?;
    with_vault(state, move |vault| {
        vault.export_csv_archive(&PathBuf::from(destination_path))
    })
    .await
}

#[tauri::command]
async fn export_xlsx(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    destination_path: String,
) -> Result<patrimoine_domain::ExportSummary, AppError> {
    require_selected_file(&app, &destination_path)?;
    with_vault(state, move |vault| {
        vault.export_xlsx(&PathBuf::from(destination_path))
    })
    .await
}

#[tauri::command]
async fn inspect_csv_import(
    app: tauri::AppHandle,
    state: State<'_, DesktopState>,
    archive_path: String,
) -> Result<RestoreInspection, AppError> {
    require_selected_file(&app, &archive_path)?;
    with_vault(state, move |vault| {
        vault.inspect_csv_import(&PathBuf::from(archive_path))
    })
    .await
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Impossible de démarrer Patrimoine : {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let data_dir = data_directory().map_err(std::io::Error::other)?;
    let vault = Vault::open(data_dir)?;
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(DesktopState(Arc::new(Mutex::new(vault))))
        .invoke_handler(tauri::generate_handler![
            get_status,
            get_settings,
            get_dashboard,
            export_dashboard_report,
            save_settings,
            list_documents,
            list_document_library_at,
            save_document_metadata,
            save_document_link,
            remove_document_link,
            replace_document,
            export_document_revision,
            open_document_revision,
            export_csv_archive,
            export_xlsx,
            inspect_csv_import,
            import_document,
            export_document,
            create_backup,
            inspect_restore,
            activate_restore,
            list_registry,
            save_entity,
            save_membership,
            save_participation,
            save_asset,
            save_ownership,
            save_valuation,
            save_company_policy,
            void_registry_record,
            get_consolidation,
            list_registry_at,
            list_registry_history,
            save_exchange_rate,
            save_profile_period,
            convert_currency,
            explain_variation,
            list_properties_at,
            save_property,
            save_rental_unit,
            save_property_event,
            dispose_property,
            get_property_report,
            list_rentals_at,
            save_lease,
            save_lease_terms,
            save_rent_due,
            save_rent_payment,
            refund_rent_payment,
            save_rent_allocation,
            save_rental_expense,
            generate_rent_dues,
            get_rental_report,
            list_treasury_at,
            list_debts_at,
            list_finance_at,
            save_treasury_account,
            save_treasury_transaction,
            save_account_reconciliation,
            save_cash_link,
            save_deposit_movement,
            post_rent_payment,
            post_rental_expense,
            post_deposit,
            post_debt_movement,
            get_treasury_report,
            save_debt_contract,
            save_debt_terms,
            save_debt_movement,
            save_debt_receivable_adjustment,
            save_debt_schedule,
            save_debt_payment_allocation,
            save_debt_asset_link,
            save_debt_guarantee,
            save_debt_coverage,
            get_debt_report,
            save_finance_coverage,
            save_rent_claim,
            save_rent_claim_adjustment,
            get_financial_report,
            list_metals_at,
            list_companies_at,
            save_metal_lot,
            save_metal_movement,
            save_metal_quote,
            save_metal_premium,
            save_metal_cash_link,
            post_metal_movement,
            transfer_metal_ownership,
            get_metal_report,
            save_company_financial_period,
            save_company_distribution,
            post_company_distribution,
            get_company_report,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}
