//! Validation for the node/RPC endpoints the user can point the wallet at.
//!
//! The settings screen used to accept any `http://` URL while its own subtitle told the user
//! to enter `https://`. Plaintext to a remote node hands anyone on the path the addresses
//! being queried, and lets them answer: balances, UTXO sets and fee estimates all arrive over
//! that channel, and a fabricated fee estimate is the cheapest way to make a wallet burn
//! money. TLS is the only thing that makes those answers trustworthy.
//!
//! Loopback is the exception. `http://127.0.0.1:8545` is how someone runs their own node, the
//! traffic never leaves the machine, and refusing it would push people back onto public
//! endpoints — the opposite of what this is for.

/// Schemes that carry no transport security. Permitted only against loopback.
const PLAINTEXT_SCHEMES: [&str; 2] = ["http://", "tcp://"];
/// Schemes that are encrypted end to end and therefore fine against any host.
const SECURE_SCHEMES: [&str; 2] = ["https://", "ssl://"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndpointError {
    /// Scheme is not one this wallet speaks.
    UnknownScheme,
    /// Plaintext scheme aimed at something other than this machine.
    InsecureRemote,
}

/// True when the authority names this machine, so plaintext never crosses a network.
///
/// Deliberately strict: only literal loopback forms count. A hostname that merely happens to
/// resolve to 127.0.0.1 today is not accepted, because what it resolves to is exactly the
/// thing an attacker on the path controls.
fn is_loopback_authority(authority: &str) -> bool {
    // Strip credentials, then the port, then any IPv6 brackets.
    let host_port = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
    let host = if let Some(rest) = host_port.strip_prefix('[') {
        // IPv6 literal: [::1]:8545
        rest.split_once(']').map_or(rest, |(h, _)| h)
    } else {
        host_port.split_once(':').map_or(host_port, |(h, _)| h)
    };
    matches!(host, "localhost" | "127.0.0.1" | "::1")
        || host.starts_with("127.")
        || host.ends_with(".localhost")
}

/// Check a user-entered endpoint.
///
/// `allow_electrum` admits the `ssl://` and `tcp://` schemes that only the Bitcoin backend
/// understands; the HTTP-only chains pass `false` so a stray `ssl://` is caught here rather
/// than failing opaquely at connect time.
///
/// An empty string is accepted: it means "use the built-in default for this network".
pub fn validate(endpoint: &str, allow_electrum: bool) -> Result<(), EndpointError> {
    let text = endpoint.trim();
    if text.is_empty() {
        return Ok(());
    }
    let lower = text.to_ascii_lowercase();

    for scheme in SECURE_SCHEMES {
        if lower.starts_with(scheme) {
            if scheme == "ssl://" && !allow_electrum {
                return Err(EndpointError::UnknownScheme);
            }
            return Ok(());
        }
    }

    for scheme in PLAINTEXT_SCHEMES {
        if lower.starts_with(scheme) {
            if scheme == "tcp://" && !allow_electrum {
                return Err(EndpointError::UnknownScheme);
            }
            let rest = &lower[scheme.len()..];
            let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
            return if is_loopback_authority(authority) {
                Ok(())
            } else {
                Err(EndpointError::InsecureRemote)
            };
        }
    }

    Err(EndpointError::UnknownScheme)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_means_use_the_default() {
        assert!(validate("", false).is_ok());
        assert!(validate("   ", false).is_ok());
    }

    #[test]
    fn https_is_always_fine() {
        assert!(validate("https://litecoinspace.org/api", false).is_ok());
        assert!(validate("HTTPS://EXAMPLE.ORG", false).is_ok());
    }

    #[test]
    fn plaintext_to_a_remote_host_is_refused() {
        assert_eq!(
            validate("http://evil.example.org:8545", false),
            Err(EndpointError::InsecureRemote)
        );
        // The interesting case: an attacker-controlled host that merely looks local.
        assert_eq!(
            validate("http://localhost.evil.example.org", false),
            Err(EndpointError::InsecureRemote)
        );
    }

    #[test]
    fn plaintext_to_loopback_is_allowed_so_self_hosting_still_works() {
        assert!(validate("http://127.0.0.1:8545", false).is_ok());
        assert!(validate("http://localhost:8899", false).is_ok());
        assert!(validate("http://[::1]:8545", false).is_ok());
        assert!(validate("http://127.0.0.1:8545/some/path", false).is_ok());
    }

    #[test]
    fn credentials_in_the_authority_do_not_smuggle_a_remote_host_past_the_check() {
        assert_eq!(
            validate("http://127.0.0.1@evil.example.org/", false),
            Err(EndpointError::InsecureRemote)
        );
    }

    #[test]
    fn electrum_schemes_only_where_they_are_understood() {
        assert!(validate("ssl://electrum.example.org:50002", true).is_ok());
        assert_eq!(
            validate("ssl://electrum.example.org:50002", false),
            Err(EndpointError::UnknownScheme)
        );
        assert!(validate("tcp://127.0.0.1:50001", true).is_ok());
        assert_eq!(
            validate("tcp://electrum.example.org:50001", true),
            Err(EndpointError::InsecureRemote)
        );
    }

    #[test]
    fn nonsense_and_dangerous_schemes_are_refused() {
        assert_eq!(validate("example.org", false), Err(EndpointError::UnknownScheme));
        assert_eq!(validate("file:///etc/passwd", false), Err(EndpointError::UnknownScheme));
        assert_eq!(validate("javascript:alert(1)", false), Err(EndpointError::UnknownScheme));
    }
}
