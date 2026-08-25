use block_wallet::configuration::wallet_display::{
    default_visible_lines, visible_text_leaks_secret,
};

#[test]
fn default_wallet_view_hides_seed_and_keys() {
    let mnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let key = "KyZpNDKnfs94vbrwhJneDi77V6jF64PWPF8x5cdJb8ifgg2DUc9d";
    let address = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";
    let visible = default_visible_lines(Some("Bitcoin"), Some(address)).join("\n");
    assert!(visible.contains(address));
    assert!(!visible_text_leaks_secret(&visible, mnemonic));
    assert!(!visible_text_leaks_secret(&visible, key));
    assert!(!visible.to_lowercase().contains("mnemonic"));
    assert!(!visible.to_lowercase().contains("private key"));
}
