use std::{fmt, str::FromStr};

use common_enums::{CardType, StandardisedCode};

pub const RETRY_STATS_KEY_VERSION: &str = "v1";
pub const KEY_SEPARATOR: char = '|';
pub const SEGMENT_SEPARATOR: char = '/';
pub const WILDCARD_SEGMENT: &str = "*";
pub const UNKNOWN_SEGMENT: &str = "UNK";

/// A single cluster-key dimension. Generic over the strict type of its resolved
/// value (`StandardisedCode` for the error code, `CardType` for the funding type,
/// `String` for the free-text issuer) so a value of the wrong kind cannot be placed
/// in the wrong slot at construction time. `Unknown` marks a value we could not
/// resolve; `Any` is the wildcard used by ancestor (root/mid) roll-up nodes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dim<T> {
    Val(T),
    Unknown,
    Any,
}

impl<T: fmt::Display> Dim<T> {
    fn as_segment(&self) -> String {
        match self {
            Self::Val(raw) => escape_segment(&raw.to_string()),
            Self::Unknown => UNKNOWN_SEGMENT.to_string(),
            Self::Any => WILDCARD_SEGMENT.to_string(),
        }
    }
}

impl<T: FromStr> Dim<T> {
    /// Build a dimension from a raw event string, normalizing the reserved
    /// spellings (empty/`*`/`UNK`) to `Unknown` and parsing the remainder into `T`.
    /// A value that does not parse into `T` also yields `Unknown`.
    pub fn from_event_value(value: Option<&str>) -> Self {
        match value.map(str::trim).filter(|v| !v.is_empty()) {
            None => Self::Unknown,
            Some(v) if v == WILDCARD_SEGMENT || v.eq_ignore_ascii_case(UNKNOWN_SEGMENT) => {
                Self::Unknown
            }
            Some(v) => T::from_str(v).map(Self::Val).unwrap_or(Self::Unknown),
        }
    }

    fn parse_segment(s: &str) -> Option<Self> {
        match s {
            WILDCARD_SEGMENT => Some(Self::Any),
            UNKNOWN_SEGMENT => Some(Self::Unknown),
            _ => {
                let decoded = unescape_segment(s)?;
                Some(
                    T::from_str(&decoded)
                        .map(Self::Val)
                        .unwrap_or(Self::Unknown),
                )
            }
        }
    }
}

fn escape_segment(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '|' => out.push_str("%7C"),
            '/' => out.push_str("%2F"),
            '*' => out.push_str("%2A"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_segment(encoded: &str) -> Option<String> {
    let mut out = String::with_capacity(encoded.len());
    let mut chars = encoded.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hi = chars.next()?;
            let lo = chars.next()?;
            let byte = u8::from_str_radix(&format!("{hi}{lo}"), 16).ok()?;
            out.push(char::from(byte));
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

/// The three-dimensional key a retry outcome is recorded under. `error_code` and
/// `card_type` are strictly typed; `issuer` is a free-text bank name resolved from
/// the card ISIN.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetryStatsClusterKey {
    pub error_code: Dim<StandardisedCode>,
    pub card_type: Dim<CardType>,
    pub issuer: Dim<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeDepth {
    Root,
    Mid,
    Leaf,
}

impl RetryStatsClusterKey {
    /// Assemble a fully-qualified (leaf) key from already-resolved dimensions.
    pub fn leaf(
        error_code: Dim<StandardisedCode>,
        card_type: Dim<CardType>,
        issuer: Dim<String>,
    ) -> Self {
        Self {
            error_code,
            card_type,
            issuer,
        }
    }

    /// Convenience constructor for callers (e.g. a retry algorithm) that already
    /// hold concrete dimension values rather than `Dim`s.
    pub fn new(
        error_code: StandardisedCode,
        card_type: CardType,
        issuer: impl Into<String>,
    ) -> Self {
        Self {
            error_code: Dim::Val(error_code),
            card_type: Dim::Val(card_type),
            issuer: Dim::Val(issuer.into()),
        }
    }

    /// Build a root node keyed on the error code alone (card_type and issuer
    /// wildcarded). This is the granularity reads currently fetch at.
    pub fn root_from_error_code(error_code: StandardisedCode) -> Self {
        Self {
            error_code: Dim::Val(error_code),
            card_type: Dim::Any,
            issuer: Dim::Any,
        }
    }

    pub fn root(&self) -> Self {
        Self {
            error_code: self.error_code.clone(),
            card_type: Dim::Any,
            issuer: Dim::Any,
        }
    }

    pub fn mid(&self) -> Self {
        Self {
            error_code: self.error_code.clone(),
            card_type: self.card_type.clone(),
            issuer: Dim::Any,
        }
    }

    pub fn depth(&self) -> NodeDepth {
        match (&self.card_type, &self.issuer) {
            (Dim::Any, Dim::Any) => NodeDepth::Root,
            (_, Dim::Any) => NodeDepth::Mid,
            _ => NodeDepth::Leaf,
        }
    }

    pub fn chain(&self) -> Vec<Self> {
        match self.depth() {
            NodeDepth::Leaf => vec![self.root(), self.mid(), self.clone()],
            NodeDepth::Mid => vec![self.root(), self.clone()],
            NodeDepth::Root => vec![self.clone()],
        }
    }

    pub fn as_db(&self) -> String {
        let mut out = String::new();
        out.push_str(RETRY_STATS_KEY_VERSION);
        out.push(KEY_SEPARATOR);
        out.push_str(&self.error_code.as_segment());
        out.push(SEGMENT_SEPARATOR);
        out.push_str(&self.card_type.as_segment());
        out.push(SEGMENT_SEPARATOR);
        out.push_str(&self.issuer.as_segment());
        out
    }

    pub fn from_db(raw: &str) -> Option<Self> {
        let (version, rest) = raw.split_once(KEY_SEPARATOR)?;
        if version != RETRY_STATS_KEY_VERSION {
            return None;
        }
        let mut parts = rest.split(SEGMENT_SEPARATOR);
        let error_code = Dim::<StandardisedCode>::parse_segment(parts.next()?)?;
        let card_type = Dim::<CardType>::parse_segment(parts.next()?)?;
        let issuer = Dim::<String>::parse_segment(parts.next()?)?;
        Some(Self {
            error_code,
            card_type,
            issuer,
        })
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn leaf_roundtrips() {
        let key = RetryStatsClusterKey::new(StandardisedCode::DoNotHonor, CardType::Credit, "HDFC");
        let raw = key.as_db();
        assert_eq!(raw, "v1|do_not_honor/CREDIT/HDFC");
        assert_eq!(RetryStatsClusterKey::from_db(&raw), Some(key));
    }

    #[test]
    fn chain_is_root_mid_leaf() {
        let chain =
            RetryStatsClusterKey::new(StandardisedCode::DoNotHonor, CardType::Credit, "HDFC")
                .chain();
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].depth(), NodeDepth::Root);
        assert_eq!(chain[1].depth(), NodeDepth::Mid);
        assert_eq!(chain[2].depth(), NodeDepth::Leaf);
        assert_eq!(chain[0].as_db(), "v1|do_not_honor/*/*");
        assert_eq!(chain[1].as_db(), "v1|do_not_honor/CREDIT/*");
        assert_eq!(chain[2].as_db(), "v1|do_not_honor/CREDIT/HDFC");
    }

    #[test]
    fn root_from_error_code_wildcards_the_rest() {
        let key = RetryStatsClusterKey::root_from_error_code(StandardisedCode::InsufficientFunds);
        assert_eq!(key.depth(), NodeDepth::Root);
        assert_eq!(key.as_db(), "v1|insufficient_funds/*/*");
    }

    #[test]
    fn issuer_delimiters_and_stars_are_percent_escaped() {
        let key =
            RetryStatsClusterKey::new(StandardisedCode::DoNotHonor, CardType::Debit, "H|D*F/C");
        let raw = key.as_db();
        assert!(raw.contains("%7C"));
        assert!(raw.contains("%2A"));
        assert!(raw.contains("%2F"));
        assert_eq!(RetryStatsClusterKey::from_db(&raw), Some(key));
    }

    #[test]
    fn from_db_rejects_foreign_versions_and_wildcards() {
        assert!(RetryStatsClusterKey::from_db("v2|a/b/c").is_none());
        assert_eq!(
            RetryStatsClusterKey::from_db("v1|do_not_honor/*/*").map(|k| k.depth()),
            Some(NodeDepth::Root)
        );
    }

    #[test]
    fn from_event_value_normalizes_reserved_spellings() {
        assert_eq!(Dim::<String>::from_event_value(Some("  ")), Dim::Unknown);
        assert_eq!(Dim::<String>::from_event_value(None), Dim::Unknown);
        assert_eq!(Dim::<String>::from_event_value(Some("*")), Dim::Unknown);
        assert_eq!(Dim::<String>::from_event_value(Some("unk")), Dim::Unknown);
        assert_eq!(
            Dim::<String>::from_event_value(Some("HDFC")),
            Dim::Val("HDFC".into())
        );
    }
}
