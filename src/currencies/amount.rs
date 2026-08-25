//! Turning what someone typed into an unambiguous decimal string.
//!
//! Every chain's amount parser used to begin `input.trim().replace(',', "")`, meaning to strip
//! thousands separators. On a phone that is a 10x hazard: in the many locales where the comma
//! *is* the decimal separator — which is what the Phosh numeric keypad offers there — someone
//! typing `1,5` for one and a half coins had it silently read as `15`.
//!
//! The rule below is chosen so that no input can ever be read as a larger number than the
//! person meant. Ambiguous input is refused outright, and a lone comma is read as a decimal
//! point, so the worst misreading available is `1,000` → `1.000`, which sends less rather
//! than more. Erring toward a smaller amount is recoverable; erring toward a larger one is not.

use crate::configuration::block_error;

/// Normalise a typed amount to a canonical `123.456` form.
///
/// Rejects anything genuinely ambiguous rather than guessing.
pub fn normalize_decimal_input(input: &str) -> Result<String, block_error::Error> {
    let trimmed: String = input.chars().filter(|c| !c.is_whitespace()).collect();
    if trimmed.is_empty() {
        return Err(block_error::Error::new("amount is required".to_string()));
    }

    let commas = trimmed.matches(',').count();
    let dots = trimmed.matches('.').count();

    // Both separators present: could be `1.234,56` or `1,234.56` depending on where the
    // person learned to write numbers, and the two differ by a factor of a thousand. There
    // is no safe guess, so ask rather than pick.
    if commas > 0 && dots > 0 {
        return Err(block_error::Error::new(
            "amount uses both ',' and '.'; enter it with a single decimal point, for example 1.5"
                .to_string(),
        ));
    }

    if commas > 1 || dots > 1 {
        return Err(block_error::Error::new(
            "amount has more than one decimal separator".to_string(),
        ));
    }

    // A single comma and nothing else is read as a decimal point. Someone who meant a
    // thousands separator gets a smaller number than intended and will see it on the review
    // screen; someone who meant a decimal point gets exactly what they meant.
    let normalized = if commas == 1 { trimmed.replace(',', ".") } else { trimmed };

    if !normalized
        .chars()
        .all(|c| c.is_ascii_digit() || c == '.')
    {
        return Err(block_error::Error::new(
            "amount must be a plain number, for example 0.001".to_string(),
        ));
    }

    Ok(normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comma_decimal_separator_is_read_as_a_decimal_point() {
        // The bug this module exists for: this used to parse as 15.
        assert_eq!(normalize_decimal_input("1,5").unwrap(), "1.5");
        assert_eq!(normalize_decimal_input("0,001").unwrap(), "0.001");
    }

    #[test]
    fn a_plain_decimal_point_is_untouched() {
        assert_eq!(normalize_decimal_input("1.5").unwrap(), "1.5");
        assert_eq!(normalize_decimal_input("0.00100000").unwrap(), "0.00100000");
        assert_eq!(normalize_decimal_input("12").unwrap(), "12");
    }

    #[test]
    fn whitespace_anywhere_is_ignored() {
        assert_eq!(normalize_decimal_input("  1.5  ").unwrap(), "1.5");
        assert_eq!(normalize_decimal_input("1 .5").unwrap(), "1.5");
    }

    #[test]
    fn mixed_separators_are_refused_rather_than_guessed() {
        assert!(normalize_decimal_input("1.234,56").is_err());
        assert!(normalize_decimal_input("1,234.56").is_err());
    }

    #[test]
    fn repeated_separators_are_refused() {
        assert!(normalize_decimal_input("1,234,567").is_err());
        assert!(normalize_decimal_input("1.2.3").is_err());
    }

    #[test]
    fn signs_exponents_and_junk_are_refused() {
        // `"+5".parse::<u64>()` succeeds in Rust, so a leading sign has to be caught here.
        assert!(normalize_decimal_input("+5").is_err());
        assert!(normalize_decimal_input("-5").is_err());
        assert!(normalize_decimal_input("1e5").is_err());
        assert!(normalize_decimal_input("0x10").is_err());
        assert!(normalize_decimal_input("abc").is_err());
    }

    #[test]
    fn empty_input_is_refused() {
        assert!(normalize_decimal_input("").is_err());
        assert!(normalize_decimal_input("   ").is_err());
    }

    #[test]
    fn no_input_is_ever_read_as_larger_than_it_looks() {
        // The safety property, stated as a test: for any accepted input containing a comma,
        // the normalised value is never larger than reading the comma as a group separator.
        for raw in ["1,5", "1,0", "9,999", "0,5"] {
            let normalized: f64 = normalize_decimal_input(raw).unwrap().parse().unwrap();
            let as_group_separator: f64 = raw.replace(',', "").parse().unwrap();
            assert!(
                normalized <= as_group_separator,
                "{raw} normalised to {normalized}, which is larger than {as_group_separator}"
            );
        }
    }
}
