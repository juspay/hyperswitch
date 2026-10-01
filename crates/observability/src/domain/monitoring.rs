//! A Grafana login derived from Router's authoritative email.

use sha2::{Digest, Sha256};

/// Namespaced to avoid matching existing email-based operational Grafana logins.
pub struct GrafanaLogin(String);

impl GrafanaLogin {
    pub fn from_router_email(email: &str) -> Option<Self> {
        if email.is_empty() || email.len() > 320 {
            return None;
        }
        Some(Self(format!("cc_{:x}", Sha256::digest(email.as_bytes()))))
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::GrafanaLogin;

    #[test]
    fn namespaced_login_uses_exact_router_email() {
        let login = |email| GrafanaLogin::from_router_email(email).map(GrafanaLogin::into_string);
        assert_eq!(login("user@example.com"), login("user@example.com"));
        assert_ne!(login("User@example.com"), login("user@example.com"));
        assert_eq!(login("user@example.com").unwrap().len(), 67);
        assert!(login("").is_none());
    }
}
