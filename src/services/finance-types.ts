import type { Fraction, RegistryRecord } from './registry-types';
import type { FxResult } from './phase4-types';

export interface FinanceCoverage extends RegistryRecord {
  valid_from: string; valid_to: string | null; assets_complete: boolean; debts_complete: boolean;
  rentals_complete: boolean; notes: string;
}
export interface RentClaimParty { entity_id: string; economic_share: Fraction | null }
export interface RentClaim extends RegistryRecord {
  due_id: string; parties: RentClaimParty[]; valid_from: string; valid_to: string | null;
  represented_by_debt_id: string | null; notes: string;
}
export interface RentClaimAdjustment extends RegistryRecord {
  due_id: string; creditor_entity_id: string; kind: string; amount: string; effective_on: string;
  source: string; notes: string;
}
export interface FinanceSnapshot {
  coverages: FinanceCoverage[]; rent_claims: RentClaim[]; rent_claim_adjustments: RentClaimAdjustment[]; knowledge_sequence: number;
}
export interface FinancialReportRequest { context: 'family' | 'person'; person_id: string | null; date: string; knowledge_sequence: number | null }
export interface FinancialWarning { code: string; object_id: string; message: string }
export interface FinancialPosition {
  object_id: string; label: string; kind: string; entity_id: string | null; country: string | null;
  currency: string; amount_original: string | null; exact_original: Fraction | null; weight: Fraction | null;
  amount_eur: string | null; exact_eur: Fraction | null; source_ids: string[]; conversion: FxResult | null;
}
export interface FinancialElimination {
  object_id: string; label: string; creditor_before_eur: string; debtor_before_eur: string;
  eliminated_eur: string; creditor_after_eur: string; debtor_after_eur: string;
  exact_eliminated: Fraction; source_ids: string[];
}
export interface FinancialReport {
  context: string; person_id: string | null; date: string; knowledge_sequence: number; status: string;
  coverage_confirmed: boolean; assets_eur: string | null; liabilities_eur: string | null; net_eur: string | null;
  exact_assets: Fraction | null; exact_liabilities: Fraction | null; exact_net: Fraction | null;
  known_assets_eur: string; known_liabilities_eur: string; known_net_eur: string;
  positions: FinancialPosition[]; eliminations: FinancialElimination[]; source_ids: string[]; warnings: FinancialWarning[];
}
