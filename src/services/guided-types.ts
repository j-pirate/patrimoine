import type { Fraction } from './registry-types';

export interface GuidedRecord {
  id: string;
  name: string;
}

export interface GuidedFamilyInput {
  request_id: string;
  date: string;
  first_name: string;
  last_name: string;
  residence_country: string | null;
}

export interface GuidedPropertyInput {
  request_id: string;
  date: string;
  name: string;
  property_type: string;
  country: string;
  city: string;
  status: string;
  estimated_value: string | null;
  currency: string;
  sole_owner_entity_id: string | null;
}

export interface GuidedAccountInput {
  request_id: string;
  date: string;
  name: string;
  kind: string;
  country: string;
  currency: string;
  opening_balance: string;
  owner_entity_id: string;
  storage_location: string;
}

export interface GuidedMetalInput {
  request_id: string;
  date: string;
  designation: string;
  metal: string;
  form: string;
  quantity: string;
  unit_weight: string;
  weight_unit: string;
  purity: string;
  owner_entity_id: string;
  country: string;
  storage_location: string;
  cost_currency: string;
}

export interface GuidedCompanyOwnerInput {
  owner_entity_id: string;
  legal_share: Fraction | null;
  economic_share: Fraction | null;
}

export interface GuidedCompanyInput {
  request_id: string;
  date: string;
  name: string;
  country: string;
  activity: string;
  owners: GuidedCompanyOwnerInput[];
}
