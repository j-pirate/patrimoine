//! Capacity is shared by every source attached to the same exact cash line.
use crate::exact::parse_decimal;
use crate::*;
use num_rational::BigRational;
use num_traits::{Signed, Zero};
use std::collections::BTreeMap;

pub fn validate_operation_cash_capacity(
    treasury: &TreasurySnapshot,
    metals: &MetalSnapshot,
    companies: &CompanySnapshot,
) -> Result<()> {
    let mut used: BTreeMap<(&str, &str), BigRational> = BTreeMap::new();
    let links = treasury
        .cash_links
        .iter()
        .map(|l| {
            (
                l.transaction_revision_id.as_str(),
                l.line_id.as_str(),
                l.amount.as_str(),
            )
        })
        .chain(metals.cash_links.iter().map(|l| {
            (
                l.transaction_revision_id.as_str(),
                l.line_id.as_str(),
                l.amount.as_str(),
            )
        }))
        .chain(
            companies
                .distributions
                .iter()
                .flat_map(|d| d.cash_links.iter())
                .map(|l| {
                    (
                        l.transaction_revision_id.as_str(),
                        l.line_id.as_str(),
                        l.amount.as_str(),
                    )
                }),
        );
    for (revision, line, amount) in links {
        *used
            .entry((revision, line))
            .or_insert_with(BigRational::zero) += parse_decimal(amount)?.abs();
    }
    for ((revision, line), total) in used {
        let entry = treasury
            .transactions
            .iter()
            .find(|t| t.revision_id == revision)
            .and_then(|t| t.entries.iter().find(|e| e.line_id == line))
            .ok_or_else(|| {
                AppError::new(
                    "validation",
                    "Un rapprochement ne référence pas une ligne de trésorerie courante.",
                )
            })?;
        if total > parse_decimal(&entry.amount)?.abs() {
            return Err(AppError::new("validation", "Les sources cumulées de loyers, financements, métaux et dividendes dépassent la capacité de la même ligne de trésorerie."));
        }
    }
    Ok(())
}
