import type { ConsolidationReport, Fraction, RegistryRecord } from './registry-types';

export interface ExchangeRate extends RegistryRecord {
  base_currency: string; quote_currency: string; rate: string; effective_on: string;
  source: string | null; notes: string;
}
export interface ProfilePeriod extends RegistryRecord {
  target_kind: 'entity' | 'asset'; target_id: string; profile_revision_id: string;
  valid_from: string; valid_to: string | null;
}
export interface RegistryHistoryEvent {
  sequence: number; recorded_at: string; object_id: string; revision_id: string;
  kind: string; label: string; reason: string; is_void: boolean;
}
export interface RegistryHistory { latest_sequence: number; events: RegistryHistoryEvent[] }
export interface FxQuery {
  amount: string; from_currency: string; to_currency: string; date: string; knowledge_sequence: number | null;
}
export interface FxWarning { code: string; message: string }
export interface FxLeg {
  rate_id: string; revision_id: string; base_currency: string; quote_currency: string; rate: string;
  from_currency: string; to_currency: string; effective_on: string; source: string | null;
  inverted: boolean; exact_rate: Fraction;
}
export interface FxResult {
  amount_original: string; from_currency: string; to_currency: string; date: string; knowledge_sequence: number;
  status: 'complete' | 'non_calculable'; method: 'identity' | 'direct' | 'inverse' | 'cross' | 'missing';
  exact_rate: Fraction | null; exact_converted: Fraction | null; amount_converted: string | null;
  legs: FxLeg[]; oldest_rate_date: string | null; age_days: number | null; warnings: FxWarning[];
}
export interface VariationRequest {
  context: 'family' | 'person'; person_id: string | null; from_date: string; to_date: string;
  knowledge_sequence: number | null;
}
export interface VariationStep {
  code: 'scope_method' | 'rights' | 'valuation' | 'fx'; label: string; amount_eur: string | null;
  exact_amount: Fraction | null; before_eur: string; after_eur: string;
  dates: { membership_date: string; method_date: string; rights_date: string; valuation_date: string; fx_date: string };
  source_ids: string[]; warnings: { code: string; object_id: string; message: string }[];
}
export interface VariationReport {
  from_date: string; to_date: string; knowledge_sequence: number;
  opening: ConsolidationReport; closing: ConsolidationReport; steps: VariationStep[];
  delta_eur: string; exact_delta: Fraction; explained_eur: string; exact_explained: Fraction;
  residual_eur: string | null; exact_residual: Fraction | null; status: string;
  warnings: { code: string; object_id: string; message: string }[];
}
