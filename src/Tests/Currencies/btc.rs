use crate::currencies::btc::*;

#[cfg(test)]
mod btc_tests {
    use crate::tests::currencies::btc::*;

    #[test]
    fn test_generate_from_mnemonic() {
        let wallet = generate_from_mnemonic("laundry kitchen trim brother survey mandate broccoli legal dance dad target \
        breeze skull indoor hurt human van way undo mesh organ art reopen jungle", "").unwrap();
        assert_eq!(wallet.private_key.unwrap(), "Kxrn8iqvbTMiM4r3Bu9Qt4fg3w3mnxus7wzeAWz65HjXopfyAwoY");
    }

    #[test]
    fn test_generate_from_private_key() {
        let wallet = generate_from_private_key("Kxrn8iqvbTMiM4r3Bu9Qt4fg3w3mnxus7wzeAWz65HjXopfyAwoY").unwrap();
        assert_eq!(wallet.public_key.unwrap(), "0263ba04267d393140886293316d2f9ca2a2dc58375949677a542221343122ce09");
    }

    #[test]
    fn test_generate_from_extended_private_key() {
        let wallet = generate_from_extended_private_key("xprvA2Y1FC2bvLV2rNHVZ9Q9vB6HVjKDjAJg6gmDnss663s6anCAdJFqvnhwEPh9Gcu1Vswrfd8dSjLYuVg2raRZXH14XBaB8n1tx7GwUHbf83G", "").unwrap();
        assert_eq!(wallet.public_key.unwrap(), "03535aa269b7fbb003e6b0919d697da62844489b7826c2dbb04a9a5d43f3755caf");
    }
}