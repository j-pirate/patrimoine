import type { FxResult } from './phase4-types';
import type { Fraction, RegistryRecord, Valuation } from './registry-types';

export interface Property extends RegistryRecord {
  asset_id: string; property_type: string; city: string; address: string; surface_m2: string | null;
  status: string; valid_from: string; valid_to: string | null; notes: string;
}
export interface RentalUnit extends RegistryRecord {
  property_asset_id: string; reference: string; name: string; surface_m2: string | null;
  valid_from: string; valid_to: string | null; available_for_rent: boolean; notes: string;
}
export interface PropertyEvent extends RegistryRecord {
  asset_id: string; kind: string; effective_on: string; price: string | null; fees: string | null;
  currency: string; notes: string;
}
export interface PropertySnapshot {
  properties: Property[]; units: RentalUnit[]; events: PropertyEvent[]; knowledge_sequence: number;
}
export interface DisposePropertyInput {
  asset_id: string; effective_on: string; price: string | null; fees: string | null;
  currency: string; reason: string; expected_sequence: number;
}
export interface PropertyReportRequest { asset_id: string; date: string; knowledge_sequence: number | null }
export interface PropertyWarning { code: string; message: string }
export interface PropertyReport {
  asset_id: string; label: string; country: string | null; date: string; knowledge_sequence: number; status: string;
  property: Property | null; units: RentalUnit[]; acquisition: PropertyEvent | null; disposal: PropertyEvent | null;
  acquisition_cost: string | null; exact_acquisition_cost: Fraction | null; acquisition_currency: string | null;
  acquisition_cost_eur: string | null; exact_acquisition_cost_eur: Fraction | null;
  valuation: Valuation | null; valuation_conversion: FxResult | null; acquisition_conversions: FxResult[];
  estimated_value_eur: string | null; exact_estimated_value_eur: Fraction | null;
  unrealized_gain_original: string | null; exact_unrealized_gain_original: Fraction | null;
  unrealized_gain_currency: string | null; unrealized_gain_eur: string | null; exact_unrealized_gain_eur: Fraction | null;
  debt_remaining_eur: string | null; net_before_disposal_eur: string | null; source_ids: string[];
  financing_conversions?: FxResult[]; exact_debt_remaining_eur?: Fraction | null; exact_net_before_disposal_eur?: Fraction | null;
  warnings: PropertyWarning[];
}
