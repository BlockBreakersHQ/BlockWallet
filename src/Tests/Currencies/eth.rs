use crate::currencies::eth::*;

#[cfg(test)]
mod btc_tests {
    use crate::tests::currencies::eth::*;

    #[test]
    fn test_generate_from_mnemonic() {
        let wallet = generate_from_mnemonic("suspect lizard spring combine convince chuckle number afraid pet soap \
        axis stairs dice toddler edit toast blue ramp street series moment blood parade hub", "").unwrap();
        assert_eq!(wallet.private_key.unwrap(), "d2a686cc01bc8b97e628aeb187666093f697fc725f3c1a830fb7b0c947f8562a");
    }

    #[test]
    fn test_generate_from_private_key() {
        let wallet = generate_from_private_key("d2a686cc01bc8b97e628aeb187666093f697fc725f3c1a830fb7b0c947f8562a").unwrap();
        assert_eq!(wallet.public_key.unwrap(), "576cba3a4637dd72d3e53fb93d6865240658c891b226efa12d94a8e1fbd048e3d8657e409437a66847a97593b98378a402619fac994e31f2db87a2aae2b1d7e4");
    }

    #[test]
    fn test_generate_from_extended_private_key() {
        let wallet = generate_from_extended_private_key("xprvA2uz2iQpDsUAZfLx8V6q95BrXbX2teNBFabDD4BkU9uegPRq7aoLQWsShvmaBQDJ128J6QzRE4qTZDUfDX6WLJykVHQDWkdKJ6jrwZuBhr2", "").unwrap();
        assert_eq!(wallet.public_key.unwrap(), "81b8d55e5ab0e3e90824f04b3c03c94e82d9ba4886d74af2aa7281ff2a78c7c8ceb7c220098b5918f68fb3000e0c2a9ac98ed89577f1b1947b1c3723d6344ac7");
    }
}