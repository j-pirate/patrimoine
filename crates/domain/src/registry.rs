//! Registry contracts: stable identities, dated rights and latest known revisions.
use crate::{AppError, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Fraction {
    pub num: String,
    pub den: String,
}

macro_rules! record {
    ($row:ident, $input:ident, {$( $(#[$meta:meta])* $field:ident: $ty:ty ),* $(,)?}) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $row {
            pub id: String,
            pub revision_id: String,
            $( $(#[$meta])* pub $field: $ty, )*
        }
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub struct $input {
            pub id: Option<String>,
            pub expected_revision_id: Option<String>,
            pub reason: String,
            $( $(#[$meta])* pub $field: $ty, )*
        }
    }
}

record!(Entity, SaveEntityInput, {
    kind: String,
    display_name: String,
    first_name: Option<String>,
    last_name: Option<String>,
    birth_date: Option<String>,
    residence_country: Option<String>,
    preferred_currency: String,
    activity: String,
    notes: String,
    archived: bool,
});
record!(Membership, SaveMembershipInput, {
    person_id: String,
    valid_from: String,
    valid_to: Option<String>,
    relationship: String,
});
record!(Participation, SaveParticipationInput, {
    owner_entity_id: String,
    company_id: String,
    legal_share: Option<Fraction>,
    economic_share: Option<Fraction>,
    voting_share: Option<Fraction>,
    valid_from: String,
    valid_to: Option<String>,
});
record!(Asset, SaveAssetInput, {
    name: String,
    country: String,
    category: String,
    notes: String,
    archived: bool,
});
record!(Ownership, SaveOwnershipInput, {
    asset_id: String,
    owner_entity_id: String,
    legal_share: Option<Fraction>,
    economic_share: Option<Fraction>,
    valid_from: String,
    valid_to: Option<String>,
});
record!(Valuation, SaveValuationInput, {
    target_kind: String,
    target_id: String,
    amount: String,
    currency: String,
    effective_on: String,
    source: String,
});
record!(CompanyPolicy, SaveCompanyPolicyInput, {
    company_id: String,
    mode: String,
    coverage_complete: bool,
    #[serde(default)] liability_qualified: bool,
    valid_from: String,
    valid_to: Option<String>,
});
record!(ExchangeRate, SaveExchangeRateInput, {
    base_currency: String,
    quote_currency: String,
    rate: String,
    effective_on: String,
    source: Option<String>,
    notes: String,
});
record!(ProfilePeriod, SaveProfilePeriodInput, {
    target_kind: String,
    target_id: String,
    profile_revision_id: String,
    valid_from: String,
    valid_to: Option<String>,
});

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryHistoryEvent {
    pub sequence: u64,
    pub recorded_at: String,
    pub object_id: String,
    pub revision_id: String,
    pub kind: String,
    pub label: String,
    pub reason: String,
    pub is_void: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryHistory {
    pub latest_sequence: u64,
    pub events: Vec<RegistryHistoryEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VoidRecordInput {
    pub id: String,
    pub expected_revision_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistrySnapshot {
    pub entities: Vec<Entity>,
    pub memberships: Vec<Membership>,
    pub participations: Vec<Participation>,
    pub assets: Vec<Asset>,
    pub ownerships: Vec<Ownership>,
    pub valuations: Vec<Valuation>,
    pub company_policies: Vec<CompanyPolicy>,
    #[serde(default)]
    pub exchange_rates: Vec<ExchangeRate>,
    #[serde(default)]
    pub profile_periods: Vec<ProfilePeriod>,
    #[serde(default)]
    pub profile_revisions: Vec<Entity>,
    #[serde(default)]
    pub asset_profile_revisions: Vec<Asset>,
    pub knowledge_sequence: u64,
}

pub type Snapshot = RegistrySnapshot;

pub fn validate_date(date: &str) -> Result<()> {
    if date.len() != 10
        || date.as_bytes()[4] != b'-'
        || date.as_bytes()[7] != b'-'
        || date
            .bytes()
            .enumerate()
            .any(|(i, b)| i != 4 && i != 7 && !b.is_ascii_digit())
    {
        return Err(AppError::new(
            "validation",
            "La date doit être au format AAAA-MM-JJ.",
        ));
    }
    let year = date[..4]
        .parse::<i32>()
        .map_err(|_| AppError::new("validation", "Année invalide."))?;
    let month = date[5..7]
        .parse::<u8>()
        .map_err(|_| AppError::new("validation", "Mois invalide."))?;
    let day = date[8..]
        .parse::<u8>()
        .map_err(|_| AppError::new("validation", "Jour invalide."))?;
    let month =
        time::Month::try_from(month).map_err(|_| AppError::new("validation", "Mois invalide."))?;
    if year < 1 || time::Date::from_calendar_date(year, month, day).is_err() {
        return Err(AppError::new(
            "validation",
            "Cette date civile n'existe pas.",
        ));
    }
    Ok(())
}

pub fn validate_interval(from: &str, to: Option<&str>) -> Result<()> {
    validate_date(from)?;
    if let Some(to) = to {
        validate_date(to)?;
        if to <= from {
            return Err(AppError::new(
                "validation",
                "La fin exclusive doit être postérieure au début.",
            ));
        }
    }
    Ok(())
}

pub fn applies(from: &str, to: Option<&str>, date: &str) -> bool {
    from <= date && to.is_none_or(|to| date < to)
}
