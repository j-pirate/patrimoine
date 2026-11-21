//! Commands that create the related records needed by a first entry together.
use crate::Fraction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GuidedRecord {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidedFamilyInput {
    pub request_id: String,
    pub date: String,
    pub first_name: String,
    pub last_name: String,
    pub residence_country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidedPropertyInput {
    pub request_id: String,
    pub date: String,
    pub name: String,
    pub property_type: String,
    pub country: String,
    pub city: String,
    pub status: String,
    pub estimated_value: Option<String>,
    pub currency: String,
    pub sole_owner_entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidedAccountInput {
    pub request_id: String,
    pub date: String,
    pub name: String,
    pub kind: String,
    pub country: String,
    pub currency: String,
    pub opening_balance: String,
    pub owner_entity_id: String,
    pub storage_location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidedMetalInput {
    pub request_id: String,
    pub date: String,
    pub designation: String,
    pub metal: String,
    pub form: String,
    pub quantity: String,
    pub unit_weight: String,
    pub weight_unit: String,
    pub purity: String,
    pub owner_entity_id: String,
    pub country: String,
    pub storage_location: String,
    pub cost_currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidedCompanyOwnerInput {
    pub owner_entity_id: String,
    pub legal_share: Option<Fraction>,
    pub economic_share: Option<Fraction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GuidedCompanyInput {
    pub request_id: String,
    pub date: String,
    pub name: String,
    pub country: String,
    pub activity: String,
    pub owners: Vec<GuidedCompanyOwnerInput>,
}
