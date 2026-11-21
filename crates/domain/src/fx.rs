//! Manual valuation FX. Quotes are units of quote currency for one base unit.
use crate::exact::{currency_minor_units, format_currency, fraction_of, parse_decimal, parse_rate};
use crate::*;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::One;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FxQuery {
    pub amount: String,
    pub from_currency: String,
    pub to_currency: String,
    pub date: String,
    pub knowledge_sequence: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxWarning {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxLeg {
    pub rate_id: String,
    pub revision_id: String,
    pub base_currency: String,
    pub quote_currency: String,
    pub rate: String,
    pub from_currency: String,
    pub to_currency: String,
    pub effective_on: String,
    pub source: Option<String>,
    pub inverted: bool,
    pub exact_rate: Fraction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxResult {
    pub amount_original: String,
    pub from_currency: String,
    pub to_currency: String,
    pub date: String,
    pub knowledge_sequence: u64,
    pub status: String,
    pub method: String,
    pub exact_rate: Option<Fraction>,
    pub exact_converted: Option<Fraction>,
    pub amount_converted: Option<String>,
    pub legs: Vec<FxLeg>,
    pub oldest_rate_date: Option<String>,
    pub age_days: Option<i64>,
    pub warnings: Vec<FxWarning>,
}

pub fn convert_currency(s: &RegistrySnapshot, q: &FxQuery) -> Result<FxResult> {
    validate_registry(s)?;
    convert_currency_unchecked(s, q)
}

fn currency(value: &str) -> Result<()> {
    if value.len() != 3 || !value.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(AppError::new(
            "validation",
            "Une devise doit être un code de trois lettres majuscules.",
        ));
    }
    Ok(())
}

fn civil_date(value: &str) -> time::Date {
    // Called only after validate_date, including persisted rate dates.
    time::Date::from_calendar_date(
        value[..4].parse().expect("validated year"),
        time::Month::try_from(value[5..7].parse::<u8>().expect("validated month"))
            .expect("validated month"),
        value[8..].parse().expect("validated day"),
    )
    .expect("validated date")
}

type FxPath = (String, BigRational, Vec<FxLeg>);

/// Request-local preparation: borrowing the immutable snapshot prevents reuse
/// after a mutation, restore or a change of knowledge sequence.
pub(crate) struct FxContext<'a> {
    snapshot: &'a RegistrySnapshot,
    rates: BTreeMap<(&'a str, &'a str), Vec<&'a ExchangeRate>>,
    paths: BTreeMap<(String, String, String), Vec<FxPath>>,
}

impl<'a> FxContext<'a> {
    pub(crate) fn new(snapshot: &'a RegistrySnapshot) -> Self {
        let mut rates: BTreeMap<_, Vec<_>> = BTreeMap::new();
        for rate in &snapshot.exchange_rates {
            rates
                .entry((rate.base_currency.as_str(), rate.quote_currency.as_str()))
                .or_default()
                .push(rate);
        }
        for rows in rates.values_mut() {
            rows.sort_by(|a, b| (&a.effective_on, &a.id).cmp(&(&b.effective_on, &b.id)));
        }
        Self {
            snapshot,
            rates,
            paths: BTreeMap::new(),
        }
    }

    fn latest(&self, from: &str, to: &str, date: &str) -> Option<&ExchangeRate> {
        let rows = self.rates.get(&(from, to))?;
        let end = rows.partition_point(|r| r.effective_on.as_str() <= date);
        end.checked_sub(1).map(|index| rows[index])
    }

    fn alternatives(&self, from: &str, to: &str, date: &str) -> Result<Vec<(BigRational, FxLeg)>> {
        let mut out = Vec::new();
        if let Some(row) = self.latest(from, to, date) {
            out.push(leg(row, from, to, false)?);
        }
        if let Some(row) = self.latest(to, from, date) {
            out.push(leg(row, from, to, true)?);
        }
        Ok(out)
    }

    fn build_paths(&self, q: &FxQuery) -> Result<Vec<FxPath>> {
        let mut paths = Vec::new();
        if q.from_currency == q.to_currency {
            paths.push(("identity".into(), BigRational::one(), Vec::new()));
        } else {
            for (rate, leg) in self.alternatives(&q.from_currency, &q.to_currency, &q.date)? {
                let method = if leg.inverted { "inverse" } else { "direct" };
                paths.push((method.into(), rate, vec![leg]));
            }
            for pivot in ["EUR", "USD", "CHF", "TND", "AED"] {
                if pivot == q.from_currency || pivot == q.to_currency {
                    continue;
                }
                for (a, first) in self.alternatives(&q.from_currency, pivot, &q.date)? {
                    for (b, second) in self.alternatives(pivot, &q.to_currency, &q.date)? {
                        paths.push(("cross".into(), &a * &b, vec![first.clone(), second]));
                    }
                }
            }
        }
        Ok(paths)
    }
}

fn leg(row: &ExchangeRate, from: &str, to: &str, inverted: bool) -> Result<(BigRational, FxLeg)> {
    let stored = parse_rate(&row.rate)?;
    let applied = if inverted { stored.recip() } else { stored };
    Ok((
        applied.clone(),
        FxLeg {
            rate_id: row.id.clone(),
            revision_id: row.revision_id.clone(),
            base_currency: row.base_currency.clone(),
            quote_currency: row.quote_currency.clone(),
            rate: row.rate.clone(),
            from_currency: from.into(),
            to_currency: to.into(),
            effective_on: row.effective_on.clone(),
            source: row.source.clone(),
            inverted,
            exact_rate: fraction_of(&applied),
        },
    ))
}

pub(crate) fn convert_currency_unchecked(s: &RegistrySnapshot, q: &FxQuery) -> Result<FxResult> {
    FxContext::new(s).convert(q)
}

impl FxContext<'_> {
    pub(crate) fn convert(&mut self, q: &FxQuery) -> Result<FxResult> {
        let s = self.snapshot;
        validate_date(&q.date)?;
        currency(&q.from_currency)?;
        currency(&q.to_currency)?;
        currency_minor_units(&q.to_currency)?;
        let amount = parse_decimal(&q.amount)?;
        if q.knowledge_sequence
            .is_some_and(|k| k != s.knowledge_sequence)
        {
            return Err(AppError::new(
                "validation",
                "La borne de connaissance ne correspond pas au registre fourni.",
            ));
        }
        let key = (
            q.from_currency.clone(),
            q.to_currency.clone(),
            q.date.clone(),
        );
        if !self.paths.contains_key(&key) {
            let paths = self.build_paths(q)?;
            self.paths.insert(key.clone(), paths);
        }
        let paths = &self.paths[&key];
        let mut out = FxResult {
            amount_original: q.amount.clone(),
            from_currency: q.from_currency.clone(),
            to_currency: q.to_currency.clone(),
            date: q.date.clone(),
            knowledge_sequence: s.knowledge_sequence,
            status: "non_calculable".into(),
            method: "missing".into(),
            exact_rate: None,
            exact_converted: None,
            amount_converted: None,
            legs: Vec::new(),
            oldest_rate_date: None,
            age_days: None,
            warnings: Vec::new(),
        };
        let Some((method, rate, legs)) = paths.first() else {
            out.warnings.push(FxWarning{code:"missing_exchange_rate".into(),message:format!("Aucun chemin de change autorisé {} → {} applicable au {} n’est connu à la séquence {}. Aucun cours futur ni parité supposée n’a été utilisé.",q.from_currency,q.to_currency,q.date,s.knowledge_sequence)});
            return Ok(out);
        };
        let exact = &amount * rate;
        out.status = "complete".into();
        out.method = method.clone();
        out.exact_rate = Some(fraction_of(rate));
        out.exact_converted = Some(fraction_of(&exact));
        out.amount_converted = Some(format_currency(&exact, &q.to_currency)?);
        out.legs = legs.clone();
        out.oldest_rate_date = legs.iter().map(|l| l.effective_on.clone()).min();
        out.age_days = out
            .oldest_rate_date
            .as_ref()
            .map(|d| (civil_date(&q.date) - civil_date(d)).whole_days());
        if out.age_days.is_some_and(|days| days > 30) {
            out.warnings.push(FxWarning{code:"stale_exchange_rate".into(),message:format!("Au {}, le cours le plus ancien du chemin date du {} ({} jours, seuil d’alerte 30 jours). Le montant reste calculé avec ce cours identifié.",q.date,out.oldest_rate_date.as_deref().unwrap_or_default(),out.age_days.unwrap_or_default())});
        }
        if paths.iter().skip(1).any(|(_, other, _)| other != rate) {
            let sources = paths
                .iter()
                .map(|(_, _, legs)| {
                    legs.iter()
                        .map(|l| format!("{}@{}", l.rate_id, l.effective_on))
                        .collect::<Vec<_>>()
                        .join(" + ")
                })
                .collect::<Vec<_>>()
                .join(" ; ");
            out.warnings.push(FxWarning{code:"inconsistent_exchange_rates".into(),message:format!("Des chemins de change divergent au {}. La priorité directe, inverse puis pivots EUR/USD/CHF/TND/AED a été appliquée sans moyenne. Sources : {}.",q.date,sources)});
        }
        Ok(out)
    }
}

pub(crate) fn converted_rational(result: &FxResult) -> Option<BigRational> {
    result.exact_converted.as_ref().map(|f| {
        BigRational::new(
            BigInt::parse_bytes(f.num.as_bytes(), 10).expect("internal exact numerator"),
            BigInt::parse_bytes(f.den.as_bytes(), 10).expect("internal exact denominator"),
        )
    })
}
