import type { Fraction, RegistryRecord, SaveRecord } from './registry-types';
import type { FxResult } from './phase4-types';
import type { RentPayment, RentalExpense } from './rental-types';

export interface TreasuryAccount extends RegistryRecord {
  asset_id: string; kind: string; currency: string; tracking_start: string;
  legal_owner_entity_id: string; bank_name: string; reference: string; country: string;
  storage_location: string; notes: string; archived: boolean;
}
export interface TreasuryEntry { line_id: string; account_id: string; amount: string; role: string }
export interface TreasuryTransaction extends RegistryRecord {
  kind: string; effective_on: string; label: string; reference: string; notes: string;
  entries: TreasuryEntry[];
}
export interface AccountReconciliation extends RegistryRecord {
  account_id: string; effective_on: string; statement_balance: string;
  statement_reference: string; document_id: string | null; notes: string;
}
export interface CashLink extends RegistryRecord {
  source_kind: string; source_id: string; source_revision_id: string;
  transaction_revision_id: string; line_id: string; amount: string;
  component: string; side: string; notes: string;
}
export interface DepositMovement extends RegistryRecord {
  lease_id: string; kind: string; amount: string; currency: string; effective_on: string;
  recipient_entity_id: string; due_id: string | null; component: string; notes: string;
}
export interface TreasurySnapshot {
  accounts: TreasuryAccount[]; transactions: TreasuryTransaction[]; reconciliations: AccountReconciliation[];
  cash_links: CashLink[]; deposits: DepositMovement[]; knowledge_sequence: number;
}
export interface CashPostingInput {
  transaction: SaveRecord<TreasuryTransaction> | null; existing_transaction_revision_id: string | null; line_id: string;
}
export interface PostedRentAllocation { due_id: string; component: string; amount: string }
export interface PostRentPaymentInput {
  expected_sequence: number; payment: SaveRecord<RentPayment>; allocations: PostedRentAllocation[]; cash: CashPostingInput;
}
export interface PostRentalExpenseInput {
  expected_sequence: number; expense: SaveRecord<RentalExpense>; cash: CashPostingInput;
}
export interface PostDepositInput {
  expected_sequence: number; movement: SaveRecord<DepositMovement>; cash: CashPostingInput | null;
}
export interface TreasuryReportRequest { account_id: string; date: string; knowledge_sequence: number | null }
export interface TreasuryWarning { code: string; object_id: string; message: string }
export interface ExecutedFee { currency: string; amount: Fraction }
export interface ExecutedFx {
  transaction_id: string; revision_id: string; effective_on: string; from_currency: string; to_currency: string;
  debited_amount: string; credited_amount: string; exact_rate: Fraction; fees: ExecutedFee[];
}
export interface ReconciliationResult {
  reconciliation_id: string; revision_id: string; effective_on: string; statement_balance: string;
  calculated_balance: string | null; difference: string | null; exact_difference: Fraction | null;
}
export interface TreasuryReport {
  account_id: string; asset_id: string; date: string; knowledge_sequence: number; currency: string; status: string;
  balance_original: string | null; exact_balance: Fraction | null; balance_eur: string | null;
  exact_balance_eur: Fraction | null; known_movement_total: Fraction; overdraft: boolean;
  opening_transaction_id: string | null; executed_fx: ExecutedFx[]; reconciliations: ReconciliationResult[];
  conversions: FxResult[]; source_ids: string[]; warnings: TreasuryWarning[];
}
