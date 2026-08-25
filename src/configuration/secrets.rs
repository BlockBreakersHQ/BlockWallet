use zeroize::Zeroize;

pub fn wipe_optional_string(value: &mut Option<String>) {
    if let Some(secret) = value {
        secret.zeroize();
    }
    *value = None;
}

pub fn wipe_vec(bytes: &mut Vec<u8>) {
    bytes.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipe_optional_string_clears_value() {
        let mut secret = Some(String::from("abandon abandon"));
        wipe_optional_string(&mut secret);
        assert!(secret.is_none());
    }
}
