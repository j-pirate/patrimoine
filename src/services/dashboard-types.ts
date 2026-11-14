import type { Fraction } from './registry-types';
import type { FinancialPosition, FinancialReport, FinancialWarning } from './finance-types';
import type { CompanyReport } from './companies-types';
import type { RentalReport } from './rental-types';
import type { VariationReport } from './phase4-types';

export interface DashboardFilters {
  country: string | null; currency: string | null; category: string | null;
  owner_entity_id: string | null; query: string;
}
export interface DashboardRequest {
  context: 'family' | 'person' | 'company'; entity_id: string | null;
  date: string; from_date: string; interval: 'monthly' | 'annual';
  knowledge_sequence: number | null; filters: DashboardFilters;
}
export interface DashboardTotals {
  known_assets_eur: string; known_liabilities_eur: string; known_net_eur: string;
  exact_known_assets: Fraction; exact_known_liabilities: Fraction; exact_known_net: Fraction;
  assets_eur: string | null; liabilities_eur: string | null; net_eur: string | null;
  position_count: number; unknown_count: number;
}
export interface DashboardPosition {
  position: FinancialPosition; side: 'asset' | 'liability' | 'contingent';
  category: string; country: string | null; owner_entity_ids: string[];
  amount_eur: string | null; exact_eur: Fraction | null;
  eliminated_eur: string; exact_eliminated: Fraction;
}
export interface DashboardBucket {
  key: string; label: string; known_assets_eur: string; known_liabilities_eur: string;
  known_net_eur: string; exact_assets: Fraction; exact_liabilities: Fraction; exact_net: Fraction;
  position_count: number; unknown_count: number;
}
export interface DashboardMember { entity_id: string; label: string; totals: DashboardTotals }
export interface DashboardPoint {
  date: string; status: string; totals: DashboardTotals;
  source_ids: string[]; warnings: FinancialWarning[];
}
export interface DashboardChange {
  from_date: string; to_date: string; known_net_change_eur: string;
  exact_known_net_change: Fraction; net_change_eur: string | null;
}
export interface DashboardReport {
  request: DashboardRequest; knowledge_sequence: number; engine_version: string;
  financial: FinancialReport | null; company: CompanyReport | null; rental: RentalReport | null;
  totals: DashboardTotals | null; positions: DashboardPosition[];
  by_category: DashboardBucket[]; by_country: DashboardBucket[]; by_currency: DashboardBucket[];
  members: DashboardMember[]; history: DashboardPoint[]; period_change: DashboardChange | null;
  variation: VariationReport | null; warnings: FinancialWarning[]; source_ids: string[];
  filters_applied: boolean;
}
