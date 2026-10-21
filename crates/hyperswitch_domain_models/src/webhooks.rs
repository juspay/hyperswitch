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
