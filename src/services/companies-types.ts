import type { Asset, CompanyPolicy, Entity, Fraction, Ownership, Participation, RegistryRecord, SaveRecord } from './registry-types';
import type { TreasuryAccount, TreasuryReport, TreasuryTransaction } from './treasury-types';

export interface CompanyFinancialPeriod extends RegistryRecord {
  company_id: string; period_start: string; period_end: string; frequency: string; currency: string;
  revenue: string | null; operating_expenses: string | null; operating_result: string | null;
  net_result: string | null; indicative_cash: string | null; indicative_debt: string | null;
  important_receivables: string | null; dividends_declared: string | null;
  indicative_global_value: string | null; notes: string;
}
export interface CompanyDistributionCashLink {
  side: string; transaction_revision_id: string; line_id: string; amount: string;
}
export interface CompanyDistribution extends RegistryRecord {
  company_id: string; beneficiary_entity_id: string; kind: string; effective_on: string;
  amount: string; currency: string; related_declaration_id: string | null;
  unlinked_reason: string; notes: string; cash_links: CompanyDistributionCashLink[];
}
export interface CompanySnapshot {
  financial_periods: CompanyFinancialPeriod[]; distributions: CompanyDistribution[]; knowledge_sequence: number;
}
export interface PostedCompanyDistributionCashLink { side: string; line_id: string; amount: string }
export interface PostCompanyDistributionInput {
  expected_sequence: number; distribution: SaveRecord<CompanyDistribution>;
  transaction: SaveRecord<TreasuryTransaction> | null; existing_transaction_revision_id: string | null;
  links: PostedCompanyDistributionCashLink[];
}
export interface CompanyReportRequest { company_id: string; date: string; frequency: string | null; knowledge_sequence: number | null }
export interface CompanyWarning { code: string; object_id: string; message: string }
export interface CompanyTrend {
  metric: string; previous_period_id: string; difference_original: string;
  exact_difference: Fraction; percentage: string | null; exact_percentage: Fraction | null;
}
export interface CompanyPeriodAnalysis {
  period: CompanyFinancialPeriod; calculated_operating_result: string | null;
  operating_result_difference: string | null; trends: CompanyTrend[];
}
export interface CompanyAssetHolding { asset: Asset; ownership: Ownership }
export interface CompanyAccountBalance { account: TreasuryAccount; report: TreasuryReport }
export interface CompanyDebtPosition {
  debt_id: string; name: string; side: string; currency: string; economic_share: Fraction | null;
  amount_original: string | null; exact_amount: Fraction | null; amount_eur: string | null; source_ids: string[];
}
export interface CompanyDistributionStatus {
  distribution: CompanyDistribution; payer_cash_complete: boolean; beneficiary_cash_complete: boolean;
  paid_against_declaration: string | null; remaining_declared: string | null;
}
export interface CompanyReport {
  company_id: string; date: string; knowledge_sequence: number; status: string;
  company: Entity; policy: CompanyPolicy | null; owners: Participation[]; subsidiaries: Participation[];
  assets: CompanyAssetHolding[]; accounts: CompanyAccountBalance[]; debt_positions: CompanyDebtPosition[];
  financial_periods: CompanyPeriodAnalysis[]; distributions: CompanyDistributionStatus[];
  source_ids: string[]; warnings: CompanyWarning[];
}
