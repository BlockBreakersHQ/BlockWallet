pub fn address_line(address: Option<&str>) -> String {
    match address.map(str::trim).filter(|value| !value.is_empty()) {
        Some(address) => format!("Address: {address}"),
        None => "Address unavailable".to_string(),
    }
}

pub fn truncate_address(address: &str) -> String {
    let chars: Vec<char> = address.chars().collect();
    if chars.len() <= 20 {
        return address.to_string();
    }
    let start: String = chars.iter().take(8).collect();
    let end: String = chars.iter().rev().take(6).rev().collect();
    format!("{start}…{end}")
}

pub fn default_visible_lines(name: Option<&str>, address: Option<&str>) -> Vec<String> {
    vec![
        name.map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Unnamed wallet")
            .to_string(),
        address_line(address),
    ]
}

pub fn visible_text_leaks_secret(visible: &str, secret: &str) -> bool {
    let secret = secret.trim();
    if secret.len() < 8 {
        return false;
    }
    visible.contains(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABANDON: &str =
        "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    const ADDRESS: &str = "bc1qcr8te4kr609gcawutmrza0j4xv80jy8z306fyu";
    const KEY: &str = "KyZpNDKnfs94vbrwhJneDi77V6jF64PWPF8x5cdJb8ifgg2DUc9d";

    #[test]
    fn default_lines_are_name_and_address_only() {
        let lines = default_visible_lines(Some("Bitcoin"), Some(ADDRESS));
        let visible = lines.join("\n");
        assert_eq!(lines[0], "Bitcoin");
        assert!(visible.contains(ADDRESS));
        assert!(!visible_text_leaks_secret(&visible, ABANDON));
        assert!(!visible_text_leaks_secret(&visible, KEY));
        assert!(!visible.to_lowercase().contains("mnemonic"));
        assert!(!visible.to_lowercase().contains("private"));
    }

    #[test]
    fn truncate_address_keeps_short_values() {
        assert_eq!(truncate_address("bc1qshort"), "bc1qshort");
        let truncated = truncate_address(ADDRESS);
        assert!(truncated.starts_with("bc1qcr8t"));
        assert!(truncated.ends_with("306fyu"));
        assert!(truncated.contains('…'));
        assert!(!truncated.contains(&ADDRESS[10..20]));
    }
}
