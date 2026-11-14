//! Exact source parsing and final financial rounding, without binary floats.
//!
//! Decimal source strings are canonical: `0`, `12`, `-12.25`, `0.001`.
//! Fractional trailing zeroes, a leading plus and negative zero are rejected.
//! Internal rational results intentionally have no source-size restriction.

use crate::{AppError, Fraction, Result};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

const MONEY_DIGITS: usize = 24;
const MONEY_SCALE: usize = 8;
const FRACTION_DIGITS: usize = 36;

fn invalid_decimal() -> AppError {
    AppError::new(
        "validation",
        "Le montant doit être un décimal canonique : au plus 24 chiffres significatifs et 8 décimales, sans exposant ni zéros superflus.",
    )
}

fn invalid_fraction() -> AppError {
    AppError::new(
        "validation",
        "Le droit doit être une fraction réduite entre 0 et 1, avec numérateur et dénominateur entiers positifs ou nuls de 36 chiffres au plus, et dénominateur strictement positif.",
    )
}

/// Form boundary only: French decimal separator and insignificant trailing
/// zeroes are accepted here; persisted/imported sources remain canonical.
pub fn normalize_decimal_input(source: &str) -> Result<String> {
    if source.len() > 64 {
        return Err(invalid_decimal());
    }
    let source = source.trim();
    if source.bytes().filter(|b| *b == b',' || *b == b'.').count() > 1 {
        return Err(invalid_decimal());
    }
    if source.contains(',') && source.contains('.') {
        return Err(invalid_decimal());
    }
    let mut normalized = source.replace(',', ".");
    if normalized.contains('.') {
        normalized = normalized
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned();
    }
    if normalized == "-0" {
        normalized = "0".into();
    }
    parse_decimal(&normalized)?;
    Ok(normalized)
}

fn canonical_unsigned(value: &str, maximum_digits: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum_digits
        && value.bytes().all(|digit| digit.is_ascii_digit())
        && (value == "0" || !value.starts_with('0'))
}

/// Parses a bounded, canonical monetary source into an exact rational value.
pub fn parse_decimal(source: &str) -> Result<BigRational> {
    parse_bounded_decimal(source, MONEY_DIGITS, MONEY_SCALE)
}

fn parse_bounded_decimal(source: &str, digits: usize, scale: usize) -> Result<BigRational> {
    // A bound before parsing avoids allocating unbounded integer inputs.
    if source.is_empty() || source.len() > 64 {
        return Err(invalid_decimal());
    }
    let (negative, unsigned) = match source.strip_prefix('-') {
        Some(value) => (true, value),
        None => (false, source),
    };
    let mut parts = unsigned.split('.');
    let integer = parts.next().ok_or_else(invalid_decimal)?;
    let fraction = parts.next();
    if parts.next().is_some() || !canonical_unsigned(integer, digits) {
        return Err(invalid_decimal());
    }
    let fractional_digits = match fraction {
        Some(value)
            if !value.is_empty()
                && value.len() <= scale
                && value.bytes().all(|digit| digit.is_ascii_digit())
                && !value.ends_with('0') =>
        {
            value
        }
        Some(_) => return Err(invalid_decimal()),
        None => "",
    };
    let significant_digits = if integer == "0" {
        fractional_digits.trim_start_matches('0').len()
    } else {
        integer.len() + fractional_digits.len()
    };
    if significant_digits > digits || (negative && integer == "0" && fractional_digits.is_empty()) {
        return Err(invalid_decimal());
    }
    let mut numerator = BigInt::parse_bytes(format!("{integer}{fractional_digits}").as_bytes(), 10)
        .ok_or_else(invalid_decimal)?;
    if negative {
        numerator = -numerator;
    }
    let denominator = BigInt::from(10_u8).pow(fractional_digits.len() as u32);
    Ok(BigRational::new(numerator, denominator))
}

fn invalid_rate() -> AppError {
    AppError::new("validation", "Le taux doit être un décimal canonique strictement positif, avec au plus 36 chiffres significatifs et 18 décimales.")
}

/// Manual FX sources have their own exact precision budget, separate from money.
pub fn parse_rate(source: &str) -> Result<BigRational> {
    let value = parse_bounded_decimal(source, 36, 18).map_err(|_| invalid_rate())?;
    if value <= BigRational::zero() {
        return Err(invalid_rate());
    }
    Ok(value)
}

pub fn normalize_rate_input(source: &str) -> Result<String> {
    if source.len() > 64 {
        return Err(invalid_rate());
    }
    let source = source.trim();
    if source.bytes().filter(|b| *b == b',' || *b == b'.').count() > 1 {
        return Err(invalid_rate());
    }
    let mut normalized = source.replace(',', ".");
    if normalized.contains('.') {
        normalized = normalized
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_owned();
    }
    parse_rate(&normalized)?;
    Ok(normalized)
}

fn bounded_fraction(source: &Fraction) -> Result<BigRational> {
    if !canonical_unsigned(&source.num, FRACTION_DIGITS)
        || !canonical_unsigned(&source.den, FRACTION_DIGITS)
        || source.den == "0"
    {
        return Err(invalid_fraction());
    }
    let numerator = BigInt::parse_bytes(source.num.as_bytes(), 10).ok_or_else(invalid_fraction)?;
    let denominator =
        BigInt::parse_bytes(source.den.as_bytes(), 10).ok_or_else(invalid_fraction)?;
    if numerator > denominator {
        return Err(invalid_fraction());
    }
    Ok(BigRational::new(numerator, denominator))
}

/// Normalizes a bounded source ratio, useful at an explicit form boundary.
/// Persisted/imported canonical ratios should be checked with `parse_fraction`.
pub fn normalize_fraction(source: &Fraction) -> Result<Fraction> {
    Ok(fraction_of(&bounded_fraction(source)?))
}

/// Requires a reduced ratio in `[0,1]`; an unknown ratio is represented by None
/// by its caller, never by a fabricated zero here.
pub fn parse_fraction(source: &Fraction) -> Result<BigRational> {
    let value = bounded_fraction(source)?;
    if value.numer().to_string() != source.num || value.denom().to_string() != source.den {
        return Err(invalid_fraction());
    }
    Ok(value)
}

/// Serializes an exact result, including negative monetary contributions and
/// large intermediate denominators. It deliberately does not reapply the
/// bounds or positivity rule intended for source ownership rights.
pub fn fraction_of(value: &BigRational) -> Fraction {
    Fraction {
        num: value.numer().to_string(),
        den: value.denom().to_string(),
    }
}

/// Final EUR display: one exact half-even rounding to two decimal places.
/// Values that round to zero always display `0.00`, without a negative zero.
pub fn format_eur(value: &BigRational) -> String {
    format_minor_units(value, 2)
}

/// Explicit minor units for the currencies supported by the local reference
/// table. Unknown codes must never silently acquire an assumed precision.
pub fn currency_minor_units(currency: &str) -> Result<u32> {
    match currency {
        "EUR" | "USD" | "CHF" | "AED" => Ok(2),
        "TND" => Ok(3),
        _ => Err(AppError::new("validation", "La devise cible n’a pas de précision monétaire prise en charge (EUR, USD, CHF, AED, TND).")),
    }
}

/// All calculations remain exact before this final half-even rounding.
pub fn format_currency(value: &BigRational, currency: &str) -> Result<String> {
    Ok(format_minor_units(value, currency_minor_units(currency)?))
}

fn format_minor_units(value: &BigRational, scale: u32) -> String {
    let factor = BigInt::from(10_u8).pow(scale);
    let scaled = value.numer().abs() * &factor;
    let denominator = value.denom();
    let mut cents = &scaled / denominator;
    let remainder = &scaled % denominator;
    let doubled_remainder = remainder * BigInt::from(2_u8);
    if doubled_remainder > *denominator
        || (doubled_remainder == *denominator && (&cents % BigInt::from(2_u8)) == BigInt::one())
    {
        cents += BigInt::one();
    }
    let sign = if value.is_negative() && !cents.is_zero() {
        "-"
    } else {
        ""
    };
    let whole = &cents / &factor;
    if scale == 0 {
        return format!("{sign}{whole}");
    }
    let fractional = (&cents % &factor).to_string();
    format!(
        "{sign}{whole}.{fractional:0>width$}",
        width = scale as usize
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ratio(num: &str, den: &str) -> Fraction {
        Fraction {
            num: num.into(),
            den: den.into(),
        }
    }

    #[test]
    fn decimal_sources_keep_exact_precision_at_the_bounds() {
        assert_eq!(parse_decimal("0").unwrap(), BigRational::zero());
        assert_eq!(
            parse_decimal("-12.25").unwrap(),
            BigRational::new(BigInt::from(-49), BigInt::from(4))
        );
        assert_eq!(
            fraction_of(&parse_decimal("0.00000001").unwrap()),
            ratio("1", "100000000")
        );
        assert!(parse_decimal("999999999999999999999999").is_ok());
        assert!(parse_decimal("1234567890123456.12345678").is_ok());
        assert!(parse_decimal("12345678901234567.12345678").is_err());
        assert!(parse_decimal("1000000000000000000000000").is_err());
        assert!(parse_decimal("0.000000001").is_err());
    }

    #[test]
    fn noncanonical_or_nonfinite_decimal_sources_are_rejected() {
        for invalid in [
            "", " ", "+1", "-0", "00", "01", "00.1", "1.", ".1", "0.0", "1.00", "1.20", "1e3",
            "NaN", "inf", "-inf", "1,25", "1 000", "--1", "1.2.3", "１", "1\n",
        ] {
            assert!(parse_decimal(invalid).is_err(), "accepted {invalid:?}");
        }
        assert!(parse_decimal(&"0".repeat(100_000)).is_err());
    }

    #[test]
    fn fractions_are_exact_reduced_and_bounded_sources() {
        let third = parse_fraction(&ratio("1", "3")).unwrap();
        assert_eq!(&third + &third + &third, BigRational::one());
        assert_eq!(fraction_of(&third), ratio("1", "3"));
        assert!(parse_fraction(&ratio("0", "1")).is_ok());
        assert!(parse_fraction(&ratio("1", "1")).is_ok());
        assert!(parse_fraction(&ratio("1", &"9".repeat(36))).is_ok());
        assert!(parse_fraction(&ratio("1", &"9".repeat(37))).is_err());
        for invalid in [
            ratio("-1", "3"),
            ratio("1", "0"),
            ratio("1", "-3"),
            ratio("2", "1"),
            ratio("2", "6"),
            ratio("0", "3"),
            ratio("01", "3"),
            ratio("1", "03"),
            ratio("1.0", "3"),
            ratio("", "1"),
        ] {
            assert!(parse_fraction(&invalid).is_err(), "accepted {invalid:?}");
        }
        assert_eq!(
            normalize_fraction(&ratio("50", "100")).unwrap(),
            ratio("1", "2")
        );
    }

    #[test]
    fn half_even_rounding_is_symmetric_and_has_no_negative_zero() {
        for (source, expected) in [
            ("1.005", "1.00"),
            ("1.015", "1.02"),
            ("1.025", "1.02"),
            ("1.035", "1.04"),
            ("-1.005", "-1.00"),
            ("-1.015", "-1.02"),
            ("-0.005", "0.00"),
            ("-0.006", "-0.01"),
            ("0.999", "1.00"),
            ("9999999999999999.99999999", "10000000000000000.00"),
        ] {
            assert_eq!(
                format_eur(&parse_decimal(source).unwrap()),
                expected,
                "{source}"
            );
        }
        assert_eq!(
            format_eur(&parse_fraction(&ratio("1", "3")).unwrap()),
            "0.33"
        );
    }

    #[test]
    fn internal_arithmetic_has_no_source_rounding_or_size_limit() {
        let third_cent = parse_decimal("0.01").unwrap() / BigInt::from(3);
        assert_eq!(
            format_eur(&(&third_cent + &third_cent + &third_cent)),
            "0.01"
        );
        assert_eq!(format_eur(&third_cent), "0.00");
        let huge = BigRational::new(BigInt::one(), BigInt::from(10_u8).pow(80) + BigInt::one());
        assert!(fraction_of(&huge).den.len() > FRACTION_DIGITS);
        assert_eq!(format_eur(&huge), "0.00");
    }
}
