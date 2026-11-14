import type { Fraction, RegistryRecord, SaveRecord } from './registry-types';
import type { FxResult } from './phase4-types';

export interface Lease extends RegistryRecord {
  unit_id: string; tenant_entity_id: string; valid_from: string; valid_to: string | null; notes: string;
}
export interface LeaseTerms extends RegistryRecord {
  lease_id: string; rent: string; charges: string; currency: string; frequency: string; due_day: number;
  deposit_required: string; valid_from: string; valid_to: string | null; notes: string;
}
export interface RentDue extends RegistryRecord {
  lease_id: string; terms_revision_id: string; period_start: string; period_end: string; due_on: string;
  rent: string; charges: string; currency: string; generation_key: string; notes: string;
}
export interface RentPayment extends RegistryRecord {
  lease_id: string; kind: string; amount: string; currency: string; effective_on: string;
  original_payment_id: string | null; recipient_entity_id: string; reference: string; unlinked_reason: string;
}
export interface RentAllocation extends RegistryRecord {
  payment_id: string; due_id: string; component: string; amount: string; currency: string; effective_on: string;
}
export interface RentalExpense extends RegistryRecord {
  asset_id: string; kind: string; amount: string; currency: string; effective_on: string;
  paid_on: string | null; recoverable: boolean; notes: string;
}
export interface RentalSnapshot {
  leases: Lease[]; lease_terms: LeaseTerms[]; lease_term_revisions: LeaseTerms[]; dues: RentDue[];
  payments: RentPayment[]; allocations: RentAllocation[]; expenses: RentalExpense[]; knowledge_sequence: number;
}
export interface GenerateRentDuesInput {
  lease_id: string; from_date: string; to_date: string; expected_sequence: number; reason: string;
}
export interface RentGenerationResult { created: RentDue[]; existing_count: number; knowledge_sequence: number }
export interface RefundAllocationInput { due_id: string; component: string; amount: string }
export interface RefundRentPaymentInput { payment: SaveRecord<RentPayment>; allocations: RefundAllocationInput[]; expected_sequence: number }
export interface RentalReportRequest {
  context: 'family' | 'person' | 'property' | 'company'; person_id: string | null; asset_id: string | null; country: string | null;
  from_date: string; to_date: string; date: string; knowledge_sequence: number | null;
}
export interface RentalWarning { code: string; object_id: string; message: string }
export interface RentDueStatus {
  due_id: string; lease_id: string; asset_id: string; due_on: string; period_start: string; period_end: string;
  currency: string; rent: string; charges: string; allocated_rent: string; allocated_charges: string;
  outstanding_rent: string; outstanding_charges: string; overdue_days: number;
  settled_on: string | null; historical_late: boolean; source_ids: string[];
}
export interface RentalOccupation {
  occupied_unit_days: number; available_unit_days: number; percent: string | null; basis: string;
}
export interface RentalReport {
  from_date: string; to_date: string; date: string; knowledge_sequence: number; status: string;
  contractual_annual_rent_eur: string | null; rent_due_eur: string | null; charges_due_eur: string | null;
  rent_received_eur: string | null; charges_received_eur: string | null; outstanding_rent_eur: string | null;
  outstanding_charges_eur: string | null; unallocated_payments_eur: string | null; economic_rent_eur: string | null;
  operating_expenses_eur: string | null; noi_eur: string | null; gross_yield_percent: string | null;
  net_yield_percent: string | null; cash_flow_after_financing_eur: string | null;
  exact_totals: Record<string, Fraction>; occupation: RentalOccupation; dues: RentDueStatus[]; warnings: RentalWarning[];
  source_ids: string[]; conversions: FxResult[];
}
