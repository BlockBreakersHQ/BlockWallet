use block_wallet::configuration::onboarding::{
    confirm_created_phrase, generate_create_phrase, parse_restore_phrase, validate_password,
    OnboardingError, WordCount,
};

const ABANDON: &str =
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

#[test]
fn create_confirm_restore_and_password_checks() {
    let twelve = generate_create_phrase(WordCount::Words12).unwrap();
    let twenty_four = generate_create_phrase(WordCount::Words24).unwrap();
    assert_eq!(twelve.split_whitespace().count(), 12);
    assert_eq!(twenty_four.split_whitespace().count(), 24);

    assert_eq!(
        confirm_created_phrase(ABANDON, &format!("  {ABANDON}  ")).unwrap(),
        ABANDON
    );
    assert_eq!(
        confirm_created_phrase(ABANDON, "not a mnemonic"),
        Err(OnboardingError::PhraseMismatch)
    );
    assert_eq!(parse_restore_phrase(ABANDON).unwrap(), ABANDON);
    assert_eq!(
        parse_restore_phrase("garbage"),
        Err(OnboardingError::InvalidPhrase)
    );
    assert_eq!(
        validate_password("a-long-enough-password", "a-different-password"),
        Err(OnboardingError::PasswordMismatch)
    );
    assert!(validate_password("a-long-enough-password", "a-long-enough-password").is_ok());
    // Short passwords are refused before the mismatch check, so a PIN cannot become the
    // only thing protecting a copied wallet file.
    assert_eq!(
        validate_password("pw", "pw"),
        Err(OnboardingError::PasswordTooShort)
    );
}
