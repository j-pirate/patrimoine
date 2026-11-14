import type { Fraction, RegistryRecord, SaveRecord, Ownership } from './registry-types';
import type { FxResult } from './phase4-types';
import type { TreasuryTransaction } from './treasury-types';

export interface MetalLot extends RegistryRecord {
  asset_id: string; metal: string; form: string; designation: string; unit_weight: string;
  weight_unit: string; purity: string; tracking_start: string; cost_currency: string;
  legal_owner_entity_id: string; country: string; storage_location: string;
  valuation_basis: string; notes: string; archived: boolean;
}
export interface MetalMovement extends RegistryRecord {
  lot_id: string; kind: string; effective_on: string; quantity_change: string;
  cost_amount: string | null; trade_amount: string | null; trade_fees: string;
  trade_currency: string | null; cash_entity_id: string; counterparty_entity_id: string | null;
  country: string; storage_location: string; unlinked_reason: string; notes: string;
}
export interface MetalQuote extends RegistryRecord {
  metal: string; effective_on: string; price: string; currency: string; weight_unit: string;
  source: string; notes: string;
}
export interface MetalPremium extends RegistryRecord {
  lot_id: string; effective_on: string; kind: string; value: string; currency: string;
  discount_per_unit: string; source: string; notes: string;
}
export interface MetalCashLink extends RegistryRecord {
  movement_id: string; movement_revision_id: string; transaction_revision_id: string;
  line_id: string; amount: string; notes: string;
}
export interface MetalSnapshot {
  lots: MetalLot[]; movements: MetalMovement[]; quotes: MetalQuote[];
  premiums: MetalPremium[]; cash_links: MetalCashLink[]; knowledge_sequence: number;
}
export interface PostedMetalCashLink { line_id: string; amount: string }
export interface PostMetalMovementInput {
  expected_sequence: number; movement: SaveRecord<MetalMovement>;
  transaction: SaveRecord<TreasuryTransaction> | null; existing_transaction_revision_id: string | null;
  links: PostedMetalCashLink[];
}
export interface TransferMetalOwnershipInput {
  expected_sequence: number; lot_id: string; effective_on: string;
  ownerships: SaveRecord<Ownership>[]; reason: string;
}
export interface MetalReportRequest { lot_id: string; date: string; knowledge_sequence: number | null }
export interface MetalWarning { code: string; object_id: string; message: string }
export interface MetalCostAllocation {
  movement_id: string; revision_id: string; quantity: Fraction;
  exact_cost_original: Fraction | null; exact_cost_eur: Fraction | null;
  net_proceeds_eur: Fraction | null; realized_gain_eur: Fraction | null;
}
export interface MetalReport {
  lot_id: string; asset_id: string; date: string; knowledge_sequence: number; status: string;
  valuation_basis: string; quantity_remaining: string | null; exact_quantity: Fraction | null;
  fine_weight_g: string | null; exact_fine_weight_g: Fraction | null;
  cost_currency: string; currency: string; cost_remaining_original: string | null;
  exact_cost_remaining_original: Fraction | null; cost_remaining_eur: string | null;
  exact_cost_remaining_eur: Fraction | null; quote: MetalQuote | null; premium: MetalPremium | null;
  intrinsic_original: string | null; exact_intrinsic_original: Fraction | null;
  intrinsic_eur: string | null; exact_intrinsic_eur: Fraction | null;
  premium_eur: string | null; premium_original: string | null; exact_premium_original: Fraction | null;
  exact_premium_eur: Fraction | null; discount_eur: string | null; discount_original: string | null;
  exact_discount_original: Fraction | null; exact_discount_eur: Fraction | null;
  resale_eur: string | null; resale_original: string | null; exact_resale_original: Fraction | null;
  exact_resale_eur: Fraction | null; valuation_eur: string | null; exact_valuation_eur: Fraction | null;
  realized_gain_eur: string | null; exact_realized_gain_eur: Fraction | null;
  unrealized_gain_eur: string | null; exact_unrealized_gain_eur: Fraction | null;
  country: string; storage_location: string; cost_allocations: MetalCostAllocation[];
  conversions: FxResult[]; source_ids: string[]; warnings: MetalWarning[];
}
