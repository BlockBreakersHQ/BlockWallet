use crate::configuration::block_error;
use crate::configuration::seed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WordCount {
    Words12,
    Words24,
}

impl WordCount {
    pub fn as_u8(self) -> u8 {
        match self {
            Self::Words12 => 12,
            Self::Words24 => 24,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingError {
    EmptyPassword,
    PasswordMismatch,
    PhraseMismatch,
    InvalidPhrase,
}

impl OnboardingError {
    pub fn as_label(&self) -> &'static str {
        match self {
            Self::EmptyPassword => "Password must not be empty.",
            Self::PasswordMismatch => "Passwords do not match.",
            Self::PhraseMismatch => "Recovery phrase does not match.",
            Self::InvalidPhrase => {
                "That recovery phrase is not valid. Check the words and try again."
            }
        }
    }
}

pub fn generate_create_phrase(word_count: WordCount) -> Result<String, block_error::Error> {
    seed::generate_mnemonic_words(word_count.as_u8())
}

pub fn normalize_phrase(input: &str) -> String {
    input
        .split_whitespace()
        .map(str::to_lowercase)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn confirm_created_phrase(expected: &str, typed: &str) -> Result<String, OnboardingError> {
    let expected = seed::parse_mnemonic(expected).map_err(|_| OnboardingError::PhraseMismatch)?;
    let typed = seed::parse_mnemonic(&normalize_phrase(typed))
        .map_err(|_| OnboardingError::PhraseMismatch)?;
    if expected != typed {
        return Err(OnboardingError::PhraseMismatch);
    }
    Ok(typed)
}

pub fn parse_restore_phrase(typed: &str) -> Result<String, OnboardingError> {
    seed::parse_mnemonic(&normalize_phrase(typed)).map_err(|_| OnboardingError::InvalidPhrase)
}

pub fn validate_password(password: &str, repeat: &str) -> Result<(), OnboardingError> {
    if password.is_empty() {
        return Err(OnboardingError::EmptyPassword);
    }
    if password != repeat {
        return Err(OnboardingError::PasswordMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABANDON: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

    #[test]
    fn generate_create_phrase_honors_word_count() {
        let twelve = generate_create_phrase(WordCount::Words12).unwrap();
        let twenty_four = generate_create_phrase(WordCount::Words24).unwrap();
        assert_eq!(twelve.split_whitespace().count(), 12);
        assert_eq!(twenty_four.split_whitespace().count(), 24);
        assert!(seed::parse_mnemonic(&twelve).is_ok());
        assert!(seed::parse_mnemonic(&twenty_four).is_ok());
    }

    #[test]
    fn confirm_accepts_extra_whitespace_and_case() {
        let typed = "  ABANDON   abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon ABOUT  ";
        assert_eq!(confirm_created_phrase(ABANDON, typed).unwrap(), ABANDON);
    }

    #[test]
    fn confirm_rejects_mismatch_and_invalid() {
        assert_eq!(
            confirm_created_phrase(ABANDON, "zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo zoo wrong"),
            Err(OnboardingError::PhraseMismatch)
        );
        let other = generate_create_phrase(WordCount::Words12).unwrap();
        assert_eq!(
            confirm_created_phrase(ABANDON, &other),
            Err(OnboardingError::PhraseMismatch)
        );
        assert_eq!(
            confirm_created_phrase(ABANDON, "not a mnemonic"),
            Err(OnboardingError::PhraseMismatch)
        );
    }

    #[test]
    fn restore_parses_valid_phrase_and_rejects_garbage() {
        assert_eq!(parse_restore_phrase(&format!("\n{ABANDON}\n")).unwrap(), ABANDON);
        assert_eq!(
            parse_restore_phrase("not a mnemonic"),
            Err(OnboardingError::InvalidPhrase)
        );
    }

    #[test]
    fn password_must_be_present_and_confirmed() {
        assert_eq!(
            validate_password("", ""),
            Err(OnboardingError::EmptyPassword)
        );
        assert_eq!(
            validate_password("secret", "other"),
            Err(OnboardingError::PasswordMismatch)
        );
        assert!(validate_password("secret", "secret").is_ok());
    }
}
