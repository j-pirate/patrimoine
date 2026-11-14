import type { ExchangeRate, FxResult, ProfilePeriod } from './phase4-types';

export interface Fraction { num: string; den: string }
export interface RegistryRecord { id: string; revision_id: string }
export interface Entity extends RegistryRecord {
  kind: 'person' | 'company'; display_name: string; first_name: string | null; last_name: string | null;
  birth_date: string | null; residence_country: string | null; preferred_currency: string;
  activity: string; notes: string; archived: boolean;
}
export interface Membership extends RegistryRecord {
  person_id: string; valid_from: string; valid_to: string | null; relationship: string;
}
export interface Participation extends RegistryRecord {
  owner_entity_id: string; company_id: string; legal_share: Fraction | null;
  economic_share: Fraction | null; voting_share: Fraction | null; valid_from: string; valid_to: string | null;
}
export interface Asset extends RegistryRecord { name: string; country: string; category: string; notes: string; archived: boolean }
export interface Ownership extends RegistryRecord {
  asset_id: string; owner_entity_id: string; legal_share: Fraction | null; economic_share: Fraction | null;
  valid_from: string; valid_to: string | null;
}
export interface Valuation extends RegistryRecord {
  target_kind: 'asset' | 'company'; target_id: string; amount: string; currency: string; effective_on: string; source: string;
}
export interface CompanyPolicy extends RegistryRecord {
  company_id: string; mode: 'detailed' | 'global_equity'; coverage_complete: boolean;
  liability_qualified: boolean; valid_from: string; valid_to: string | null;
}
export interface RegistrySnapshot {
  entities: Entity[]; memberships: Membership[]; participations: Participation[]; assets: Asset[];
  ownerships: Ownership[]; valuations: Valuation[]; company_policies: CompanyPolicy[]; knowledge_sequence: number;
  exchange_rates: ExchangeRate[]; profile_periods: ProfilePeriod[];
  profile_revisions: Entity[]; asset_profile_revisions: Asset[];
}
export type SaveRecord<T extends RegistryRecord> = Omit<T, 'id' | 'revision_id'> & {
  id: string | null; expected_revision_id: string | null; reason: string;
};
export interface ConsolidationRequest {
  context: 'family' | 'person'; person_id: string | null; date: string; knowledge_sequence: number | null;
}
export interface Contribution {
  object_id: string; label: string; kind: string; country: string | null; category: string; weight: Fraction;
  amount_original: string; currency: string; valuation_id: string | null; valuation_revision_id: string | null;
  valuation_date: string | null; amount_eur: string; exact_eur: Fraction; source_ids: string[];
  conversion?: FxResult | null;
}
export interface ConsolidationReport {
  context: string; person_id: string | null; date: string; knowledge_sequence: number; engine_version: string;
  status: string; gross_eur: string; debt_eur: string; net_eur: string;
  exact_gross: Fraction; exact_debt: Fraction; exact_net: Fraction; global_equity_eur: string;
  gross_is_underlying_complete: boolean; contributions: Contribution[];
  warnings: { code: string; object_id: string; message: string }[];
}
export const emptyRegistry: RegistrySnapshot = {
  entities: [], memberships: [], participations: [], assets: [], ownerships: [], valuations: [], company_policies: [], knowledge_sequence: 0,
  exchange_rates: [], profile_periods: [], profile_revisions: [], asset_profile_revisions: [],
};
