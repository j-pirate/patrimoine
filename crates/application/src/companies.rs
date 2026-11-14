//! Company observations are analytical sources; payments use the treasury journal.
use patrimoine_domain::*;

pub trait CompanyService {
    fn list_companies_at(&self, knowledge_sequence: Option<u64>) -> Result<CompanySnapshot>;
    fn save_company_financial_period(
        &mut self,
        input: SaveCompanyFinancialPeriodInput,
    ) -> Result<CompanyFinancialPeriod>;
    fn save_company_distribution(
        &mut self,
        input: SaveCompanyDistributionInput,
    ) -> Result<CompanyDistribution>;
    fn post_company_distribution(
        &mut self,
        input: PostCompanyDistributionInput,
    ) -> Result<CompanyDistribution>;
    fn get_company_report(&self, request: CompanyReportRequest) -> Result<CompanyReport>;
}
