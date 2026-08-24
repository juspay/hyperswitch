use std::{fmt, str::FromStr};

use common_enums::{CardType, StandardisedCode};
use router_env::logger;

/// Layout version prefixed to every persisted cluster key. Bump this whenever
/// the key layout changes (segment order, segment count, escaping rules) so
/// `from_db_string` rejects keys written under an old layout instead of silently
/// misreading them.
pub const RETRY_STATS_KEY_VERSION: &str = "v1";
pub const KEY_SEPARATOR: char = '|';
pub const SEGMENT_SEPARATOR: char = '/';
pub const WILDCARD_SEGMENT: &str = "*";
pub const UNKNOWN_SEGMENT: &str = "UNK";
/// Number of dimension segments in a key: `error_code` / `card_type` / `issuer`.
/// Adding or removing a dimension is a layout change — bump
/// [`RETRY_STATS_KEY_VERSION`] together with this count.
const SEGMENT_COUNT: usize = 3;

/// A single cluster-key dimension. Generic over the strict type of its resolved
/// value (`StandardisedCode` for the error code, `CardType` for the funding type,
/// `String` for the free-text issuer) so a value of the wrong kind cannot be placed
/// in the wrong slot at construction time. `Unknown` marks a value we could not
/// resolve; `Any` is the wildcard placeholder used by ancestor roll-up nodes.
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

/// Reverse [`escape_segment`]. Percent-decoding is **byte-oriented**: a `%XX`
/// escape contributes one raw byte and literal characters contribute their UTF-8
/// bytes, then the whole buffer is validated as UTF-8 in one shot. This keeps the
/// decoder compatible with standard percent-encoding — a `%`-escaped multi-byte
/// UTF-8 sequence (e.g. `%C3%BC`) reassembles into the original character instead
/// of being mangled into one Latin-1 char per byte. Malformed escapes (short or
/// non-hex) and byte sequences that are not valid UTF-8 both yield `None`.
fn unescape_segment(encoded: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(encoded.len());
    let mut chars = encoded.chars();
    while let Some(ch) = chars.next() {
        if ch == '%' {
            let hi = chars.next()?;
            let lo = chars.next()?;
            bytes.push(u8::from_str_radix(&format!("{hi}{lo}"), 16).ok()?);
        } else {
            let mut buf = [0u8; 4];
            bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
        }
    }
    String::from_utf8(bytes).ok()
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

    /// Build the interim error-code-only key: keyed on the error code with `card_type`
    /// and `issuer` left `Unknown`, serializing to `error_code/UNK/UNK`. While those two
    /// dimensions are not being populated, this is exactly what the write path stores,
    /// so reads use it for an exact-key match.
    pub fn from_error_code(error_code: StandardisedCode) -> Self {
        Self {
            error_code: Dim::Val(error_code),
            card_type: Dim::Unknown,
            issuer: Dim::Unknown,
        }
        // TODO(revenue-recovery roll-up): once card_type/issuer are populated again, the
        // ancestor node keyed on the error code alone should instead wildcard the
        // sub-dimensions (`error_code/*/*`) so it rolls up all leaves under that code:
        //     Self { error_code: Dim::Val(error_code), card_type: Dim::Any, issuer: Dim::Any }
    }

    /// The key's dimension segments in persisted order. This array is the
    /// single source of truth for the layout shared by `as_db_string` and `from_db_string`:
    /// always edit the order HERE, never in either function by hand, and treat
    /// any order change as a breaking layout change (bump
    /// [`RETRY_STATS_KEY_VERSION`]) — existing rows are keyed by the old order.
    fn segments(&self) -> [String; SEGMENT_COUNT] {
        [
            self.error_code.as_segment(),
            self.card_type.as_segment(),
            self.issuer.as_segment(),
        ]
    }

    /// Serialize to the persisted `cluster_key`. CRITICAL: must remain the
    /// exact mirror of `from_db_string` — every segment position written here is
    /// positional, and rows already stored under a given layout decode only if
    /// both sides agree on the order in [`Self::segments`].
    pub fn as_db_string(&self) -> String {
        let mut out = String::new();
        out.push_str(RETRY_STATS_KEY_VERSION);
        out.push(KEY_SEPARATOR);
        out.push_str(&self.segments().join(&SEGMENT_SEPARATOR.to_string()));
        out
    }

    /// Parse a persisted `cluster_key`. CRITICAL: must remain the exact mirror
    /// of `as_db_string` — segment positions are bound to field names only by the
    /// order in [`Self::segments`]. Parsing is strict about arity (exactly
    /// [`SEGMENT_COUNT`] segments): a key with extra or missing segments is
    /// rejected as `None` rather than silently truncated or padded, so a
    /// miswritten row can never resolve to the wrong cluster.
    pub fn from_db_string(raw: &str) -> Option<Self> {
        let Some((version, rest)) = raw.split_once(KEY_SEPARATOR) else {
            logger::error!(
                cluster_key = raw,
                "revenue_recovery_retry_stats: cluster key has no version separator"
            );
            return None;
        };
        if version != RETRY_STATS_KEY_VERSION {
            logger::error!(
                cluster_key = raw,
                version,
                expected_version = RETRY_STATS_KEY_VERSION,
                "revenue_recovery_retry_stats: cluster key written under an unsupported layout version"
            );
            return None;
        }
        let segments = rest.split(SEGMENT_SEPARATOR).collect::<Vec<_>>();
        let Ok([error_code, card_type, issuer]) = <[&str; SEGMENT_COUNT]>::try_from(segments)
        else {
            logger::error!(
                cluster_key = raw,
                expected_segments = SEGMENT_COUNT,
                "revenue_recovery_retry_stats: cluster key has the wrong number of segments"
            );
            return None;
        };
        Some(Self {
            error_code: Dim::<StandardisedCode>::parse_segment(error_code)
                .or_else(|| Self::log_segment_failure(raw, "error_code", error_code))?,
            card_type: Dim::<CardType>::parse_segment(card_type)
                .or_else(|| Self::log_segment_failure(raw, "card_type", card_type))?,
            issuer: Dim::<String>::parse_segment(issuer)
                .or_else(|| Self::log_segment_failure(raw, "issuer", issuer))?,
        })
    }

    /// Log a segment that could not be decoded back into a `Dim` and propagate the
    /// failure as `None`. Only reachable when `unescape_segment` rejects malformed
    /// percent-escaping — parse failures on a strictly-typed value degrade to
    /// `Dim::Unknown` inside `parse_segment` rather than surfacing here.
    fn log_segment_failure<T>(raw: &str, field: &str, segment: &str) -> Option<T> {
        logger::error!(
            cluster_key = raw,
            field,
            segment,
            "revenue_recovery_retry_stats: cluster key segment failed to decode"
        );
        None
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn leaf_roundtrips() {
        let key = RetryStatsClusterKey::new(StandardisedCode::DoNotHonor, CardType::Credit, "HDFC");
        let raw = key.as_db_string();
        assert_eq!(raw, "v1|do_not_honor/CREDIT/HDFC");
        assert_eq!(RetryStatsClusterKey::from_db_string(&raw), Some(key));
    }

    #[test]
    fn issuer_delimiters_and_stars_are_percent_escaped() {
        let key =
            RetryStatsClusterKey::new(StandardisedCode::DoNotHonor, CardType::Debit, "H|D*F/C");
        let raw = key.as_db_string();
        assert!(raw.contains("%7C"));
        assert!(raw.contains("%2A"));
        assert!(raw.contains("%2F"));
        assert_eq!(RetryStatsClusterKey::from_db_string(&raw), Some(key));
    }

    #[test]
    fn non_ascii_issuer_roundtrips() {
        // Non-ASCII issuer names ride through escaping literally and must survive
        // a full serialize -> parse cycle without corruption.
        let key =
            RetryStatsClusterKey::new(StandardisedCode::DoNotHonor, CardType::Credit, "Zürich");
        assert_eq!(
            RetryStatsClusterKey::from_db_string(&key.as_db_string()),
            Some(key)
        );
    }

    #[test]
    fn standard_percent_encoded_utf8_decodes_correctly() {
        // A `%`-escaped multi-byte UTF-8 sequence (standard percent-encoding of "ü")
        // must reassemble into the original character, not one Latin-1 char per byte.
        let key = RetryStatsClusterKey::from_db_string("v1|do_not_honor/CREDIT/Z%C3%BCrich")
            .expect("valid key");
        assert_eq!(key.issuer, Dim::Val("Zürich".to_string()));
    }

    #[test]
    fn invalid_utf8_percent_sequence_is_rejected() {
        // A lone continuation byte is not valid UTF-8, so the segment must fail to
        // decode rather than producing a mojibake issuer.
        assert!(RetryStatsClusterKey::from_db_string("v1|do_not_honor/CREDIT/%FF").is_none());
    }

    #[test]
    fn from_error_code_uses_unknown_sub_dims() {
        let key = RetryStatsClusterKey::from_error_code(StandardisedCode::InsufficientFunds);
        assert_eq!(key.as_db_string(), "v1|insufficient_funds/UNK/UNK");
    }

    #[test]
    fn from_db_rejects_foreign_versions_and_wildcards() {
        assert!(RetryStatsClusterKey::from_db_string("v2|a/b/c").is_none());
        assert!(RetryStatsClusterKey::from_db_string("v1|do_not_honor/*/*").is_some());
    }

    #[test]
    fn from_db_rejects_wrong_segment_counts() {
        assert!(RetryStatsClusterKey::from_db_string("v1|a/b").is_none());
        assert!(RetryStatsClusterKey::from_db_string("v1|a/b/c/d").is_none());
        assert!(RetryStatsClusterKey::from_db_string("v1|").is_none());
        assert!(RetryStatsClusterKey::from_db_string("v1").is_none());
    }

    #[test]
    fn unknown_and_any_dims_roundtrip() {
        let key = RetryStatsClusterKey {
            error_code: Dim::Unknown,
            card_type: Dim::Any,
            issuer: Dim::Val("HDFC".into()),
        };
        assert_eq!(key.as_db_string(), "v1|UNK/*/HDFC");
        assert_eq!(
            RetryStatsClusterKey::from_db_string(&key.as_db_string()),
            Some(key)
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
