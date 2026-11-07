use std::{net::IpAddr, str::FromStr};

use ip_network::IpNetwork;

/// An incoming webhook source IP or CIDR range, validated during config deserialization.
#[derive(Debug, serde_with::DeserializeFromStr)]
pub struct IncomingWebhookIpRange(IpNetwork);

impl FromStr for IncomingWebhookIpRange {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let value = value.trim();
        value
            .parse::<IpNetwork>()
            .or_else(|_| value.parse::<IpAddr>().map(IpNetwork::from))
            .map(Self)
            .map_err(|error| {
                format!("Invalid incoming webhook allowed IP range '{value}': {error}")
            })
    }
}

impl IncomingWebhookIpRange {
    /// Whether the source IP belongs to this allowed range.
    pub fn contains(&self, source_ip: IpAddr) -> bool {
        self.0.contains(source_ip)
    }
}

#[cfg(test)]
mod tests {
    use ip_network::IpNetwork;

    use super::IncomingWebhookIpRange;

    #[test]
    fn parses_ip_addresses_and_cidr_ranges() {
        for (value, expected) in [
            ("198.51.100.10", "198.51.100.10/32"),
            (" 198.51.100.10 ", "198.51.100.10/32"),
            ("2001:db8::10", "2001:db8::10/128"),
            ("195.35.90.0/25", "195.35.90.0/25"),
            ("2001:db8::/64", "2001:db8::/64"),
        ] {
            let range = value.parse::<IncomingWebhookIpRange>().unwrap();
            assert_eq!(range.0, expected.parse::<IpNetwork>().unwrap());
        }
    }

    #[test]
    fn rejects_invalid_ips_and_cidr_ranges() {
        for value in [
            "invalid",
            "198.51.100.10:46532",
            "198.51.100.10/33",
            "198.51.100.10/24",
            "2001:db8::1/64",
        ] {
            assert!(
                value.parse::<IncomingWebhookIpRange>().is_err(),
                "accepted {value}"
            );
        }
    }
}
