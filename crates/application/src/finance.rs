//! Consolidated financial coverage and explicit historical rental claims.
use patrimoine_domain::*;

pub trait FinancialService {
    fn list_finance_at(&self, knowledge_sequence: Option<u64>) -> Result<FinanceSnapshot>;
    fn save_finance_coverage(&mut self, input: SaveFinanceCoverageInput)
        -> Result<FinanceCoverage>;
    fn save_rent_claim(&mut self, input: SaveRentClaimInput) -> Result<RentClaim>;
    fn save_rent_claim_adjustment(
        &mut self,
        input: SaveRentClaimAdjustmentInput,
    ) -> Result<RentClaimAdjustment>;
    fn get_financial_report(&self, request: FinancialReportRequest) -> Result<FinancialReport>;
}
