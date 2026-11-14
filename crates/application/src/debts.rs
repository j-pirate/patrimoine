//! Persisted financing use cases, independent from the desktop bridge.
use patrimoine_domain::*;

pub trait DebtService {
    fn list_debts_at(&self, knowledge_sequence: Option<u64>) -> Result<DebtSnapshot>;
    fn save_debt_contract(&mut self, input: SaveDebtContractInput) -> Result<DebtContract>;
    fn save_debt_terms(&mut self, input: SaveDebtTermsInput) -> Result<DebtTerms>;
    fn save_debt_movement(&mut self, input: SaveDebtMovementInput) -> Result<DebtMovement>;
    fn save_debt_receivable_adjustment(
        &mut self,
        input: SaveDebtReceivableAdjustmentInput,
    ) -> Result<DebtReceivableAdjustment>;
    fn save_debt_schedule(&mut self, input: SaveDebtScheduleInput) -> Result<DebtSchedule>;
    fn save_debt_payment_allocation(
        &mut self,
        input: SaveDebtPaymentAllocationInput,
    ) -> Result<DebtPaymentAllocation>;
    fn save_debt_asset_link(&mut self, input: SaveDebtAssetLinkInput) -> Result<DebtAssetLink>;
    fn save_debt_guarantee(&mut self, input: SaveDebtGuaranteeInput) -> Result<DebtGuarantee>;
    fn save_debt_coverage(&mut self, input: SaveDebtCoverageInput) -> Result<DebtCoverage>;
    fn get_debt_report(&self, request: DebtReportRequest) -> Result<DebtReport>;
}
