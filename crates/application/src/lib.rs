//! Cas d'usage indépendants de Tauri ; l'infrastructure réalise ce contrat.
use patrimoine_domain::*;
use std::path::Path;
mod companies;
mod dashboard;
mod debts;
mod documents;
mod finance;
mod metals;
mod portability;
mod rentals;
mod treasury;
pub use companies::CompanyService;
pub use dashboard::DashboardService;
pub use debts::DebtService;
pub use documents::DocumentService;
pub use finance::FinancialService;
pub use metals::MetalService;
pub use portability::PortabilityService;
pub use rentals::RentalService;
pub use treasury::TreasuryService;

pub trait PropertyService {
    fn list_properties_at(&self, knowledge_sequence: Option<u64>) -> Result<PropertySnapshot>;
    fn save_property(&mut self, input: SavePropertyInput) -> Result<Property>;
    fn save_rental_unit(&mut self, input: SaveRentalUnitInput) -> Result<RentalUnit>;
    fn save_property_event(&mut self, input: SavePropertyEventInput) -> Result<PropertyEvent>;
    fn dispose_property(&mut self, input: DisposePropertyInput) -> Result<PropertySnapshot>;
    fn get_property_report(&self, request: PropertyReportRequest) -> Result<PropertyReport>;
}

pub trait GuidedCreationService {
    fn read_onboarding_state(&self, generation_id: &str) -> Result<Option<String>>;
    fn write_onboarding_state(&self, generation_id: &str, data: &str) -> Result<()>;
    fn create_guided_family(&mut self, input: GuidedFamilyInput) -> Result<GuidedRecord>;
    fn create_guided_property(&mut self, input: GuidedPropertyInput) -> Result<GuidedRecord>;
    fn create_guided_account(&mut self, input: GuidedAccountInput) -> Result<GuidedRecord>;
    fn create_guided_metal(&mut self, input: GuidedMetalInput) -> Result<GuidedRecord>;
    fn create_guided_company(&mut self, input: GuidedCompanyInput) -> Result<GuidedRecord>;
}

pub trait VaultService {
    fn status(&self) -> Result<VaultStatus>;
    fn save_settings(&mut self, settings: Settings) -> Result<Settings>;
    fn list_documents(&self) -> Result<Vec<DocumentSummary>>;
    fn import_document(&mut self, source: &Path, description: &str) -> Result<DocumentSummary>;
    fn export_document(&self, id: &str, destination: &Path) -> Result<()>;
    fn create_backup(&mut self, destination: &Path) -> Result<BackupSummary>;
    fn inspect_restore(&mut self, archive: &Path) -> Result<RestoreInspection>;
    fn activate_restore(
        &mut self,
        token: &str,
        confirmed: bool,
        safety_backup: Option<&Path>,
    ) -> Result<VaultStatus>;
}

pub trait RegistryService {
    fn list_registry(&self) -> Result<RegistrySnapshot>;
    fn list_registry_at(&self, knowledge_sequence: Option<u64>) -> Result<RegistrySnapshot>;
    fn list_registry_history(&self) -> Result<RegistryHistory>;
    fn save_entity(&mut self, input: SaveEntityInput) -> Result<Entity>;
    fn save_membership(&mut self, input: SaveMembershipInput) -> Result<Membership>;
    fn save_participation(&mut self, input: SaveParticipationInput) -> Result<Participation>;
    fn save_asset(&mut self, input: SaveAssetInput) -> Result<Asset>;
    fn save_ownership(&mut self, input: SaveOwnershipInput) -> Result<Ownership>;
    fn save_valuation(&mut self, input: SaveValuationInput) -> Result<Valuation>;
    fn save_company_policy(&mut self, input: SaveCompanyPolicyInput) -> Result<CompanyPolicy>;
    fn save_exchange_rate(&mut self, input: SaveExchangeRateInput) -> Result<ExchangeRate>;
    fn save_profile_period(&mut self, input: SaveProfilePeriodInput) -> Result<ProfilePeriod>;
    fn void_registry_record(&mut self, input: VoidRecordInput) -> Result<()>;
    fn get_consolidation(&self, request: ConsolidationRequest) -> Result<ConsolidationReport>;
    fn convert_currency(&self, query: FxQuery) -> Result<FxResult>;
    fn explain_variation(&self, request: VariationRequest) -> Result<VariationReport>;
}

pub fn validate_settings(settings: &Settings) -> Result<()> {
    let name = settings.workspace_name.trim();
    if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
        return Err(AppError::new("validation", "Le nom de l’espace doit contenir entre 1 et 100 caractères, sans caractère de contrôle."));
    }
    if !["EUR", "TND", "CHF", "AED", "USD"].contains(&settings.display_currency.as_str()) {
        return Err(AppError::new(
            "validation",
            "Choisissez une devise disponible : EUR, TND, CHF, AED ou USD.",
        ));
    }
    Ok(())
}
