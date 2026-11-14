import type { Fraction, RegistryRecord, SaveRecord } from './registry-types';
import type { FxResult } from './phase4-types';
import type { TreasuryTransaction } from './treasury-types';

export interface FinancingParty {
  line_id: string; side: string; entity_id: string;
  legal_share: Fraction | null; economic_share: Fraction | null;
  liability_kind: string; notes: string;
}
export interface DebtContract extends RegistryRecord {
  name: string; currency: string; tracking_start: string; notes: string; archived: boolean;
}
export interface DebtTerms extends RegistryRecord {
  debt_id: string; valid_from: string; valid_to: string | null; principal_status: string;
  interest_rate: string | null; interest_convention: string; parties: FinancingParty[]; notes: string;
}
export interface DebtMovement extends RegistryRecord {
  debt_id: string; kind: string; effective_on: string; principal_change: string;
  interest_paid: string; fees_paid: string; unlinked_reason: string; notes: string;
}
export interface DebtReceivableAdjustment extends RegistryRecord {
  debt_id: string; creditor_entity_id: string; kind: string; effective_on: string;
  valuation_delta: string; source: string; notes: string;
}
export interface DebtSchedule extends RegistryRecord {
  debt_id: string; due_on: string; expected_principal: string; expected_interest: string;
  expected_fees: string; status: string; notes: string;
}
export interface DebtPaymentAllocation extends RegistryRecord {
  movement_id: string; schedule_id: string; effective_on: string; principal: string; interest: string; fees: string;
}
export interface DebtAssetLink extends RegistryRecord {
  debt_id: string; asset_id: string; relation_kind: string; attributed_principal_share: Fraction | null;
  valid_from: string; valid_to: string | null; notes: string;
}
export interface DebtGuarantee extends RegistryRecord {
  debt_id: string; guarantor_entity_id: string; limit_amount: string | null;
  valid_from: string; valid_to: string | null; notes: string;
}
export interface DebtCoverage extends RegistryRecord {
  debt_id: string; company_id: string; side: string; valid_from: string; valid_to: string | null;
  included: boolean; source: string; notes: string;
}
export interface DebtSnapshot {
  contracts: DebtContract[]; terms: DebtTerms[]; movements: DebtMovement[];
  receivable_adjustments: DebtReceivableAdjustment[]; schedules: DebtSchedule[];
  payment_allocations: DebtPaymentAllocation[]; asset_links: DebtAssetLink[];
  guarantees: DebtGuarantee[]; coverages: DebtCoverage[]; knowledge_sequence: number;
}
export interface DebtReportRequest { debt_id: string; date: string; knowledge_sequence: number | null }
export interface DebtWarning { code: string; object_id: string; message: string }
export interface DebtBalance {
  debt_id: string; date: string; currency: string; principal_original: string | null;
  exact_principal: Fraction | null; interest_paid_original: string; exact_interest_paid: Fraction;
  fees_paid_original: string; exact_fees_paid: Fraction; opening_known: boolean;
  source_ids: string[]; warnings: DebtWarning[];
}
export interface DebtPartyPosition {
  party: FinancingParty; nominal_original: string | null; exact_nominal: Fraction | null;
  adjustment_original: string; exact_adjustment: Fraction; carrying_value_original: string | null;
  exact_carrying_value: Fraction | null; legal_maximum_original: string | null;
}
export interface DebtScheduleStatus {
  schedule: DebtSchedule; paid_principal: string; paid_interest: string; paid_fees: string;
  remaining_principal: string; remaining_interest: string; remaining_fees: string;
  overdue_days: number; settled_on: string | null; historical_late: boolean;
}
export interface DebtReport {
  debt_id: string; date: string; knowledge_sequence: number; status: string;
  contract: DebtContract; terms: DebtTerms | null; balance: DebtBalance;
  principal_eur: string | null; exact_principal_eur: Fraction | null; conversion: FxResult | null;
  parties: DebtPartyPosition[]; schedules: DebtScheduleStatus[]; asset_links: DebtAssetLink[];
  guarantees: DebtGuarantee[]; coverages: DebtCoverage[]; source_ids: string[]; warnings: DebtWarning[];
}
export interface PostDebtLinkInput { line_id: string; side: string; component: string; amount: string }
export interface PostDebtMovementInput {
  expected_sequence: number; movement: SaveRecord<DebtMovement>;
  transaction: SaveRecord<TreasuryTransaction> | null; existing_transaction_revision_id: string | null;
  links: PostDebtLinkInput[];
}
