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

/// Compile-time guard invoked once per generated key: asserts the dimension
/// indices are exactly `0, 1, 2, …` in declaration order. This is what turns the
/// explicit position numbers in [`define_retry_stats_key!`] from decoration into
/// an enforced invariant — a reorder, a gap, or a duplicated index makes this
/// `assert!` fail at compile time rather than silently shifting the wire layout.
#[allow(clippy::indexing_slicing)]
const fn assert_contiguous_indices(indices: &[usize]) {
    let mut i = 0;
    while i < indices.len() {
        assert!(
            indices[i] == i,
            "retry-stats cluster key: dimension indices must be 0, 1, 2, … in \
             declaration order. Do NOT reorder or insert dimensions — only append \
             a new one with the next index, and bump RETRY_STATS_KEY_VERSION."
        );
        i += 1;
    }
}

/// Declares the cluster key's dimensions **once**, in persisted order, and
/// generates the struct plus the entire serialize/deserialize surface from that
/// single ordered list: the `Dim<_>` fields, `SEGMENT_COUNT`, `segments`,
/// `leaf`/`new`, and the exact-mirror `as_db_string` / `from_db_string` pair.
///
/// Each dimension carries an explicit `<index> =>` position. The index is not
/// cosmetic: [`assert_contiguous_indices`] enforces at compile time that the
/// indices run `0, 1, 2, …` in the order the fields are written, so the hazards
/// called out at the invocation site turn into build failures instead of silent
/// data corruption.
macro_rules! define_retry_stats_key {
    (
        $(#[$struct_meta:meta])*
        $vis:vis struct $name:ident {
            $(
                $(#[$field_meta:meta])*
                $idx:literal => $field:ident : $ty:ty
            ),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        #[derive(Clone, Debug, PartialEq, Eq)]
        $vis struct $name {
            $(
                $(#[$field_meta])*
                pub $field: Dim<$ty>,
            )*
        }

        /// Number of dimension segments in a key, derived from the dimension list
        /// so it can never drift from the number of fields.
        const SEGMENT_COUNT: usize = [$( $idx ),*].len();

        // Compile-time check that the declared indices are `0, 1, 2, ...` in order.
        const _: () = assert_contiguous_indices(&[$( $idx ),*]);

        impl $name {
            /// Assemble a fully-qualified (leaf) key from already-resolved dimensions.
            pub fn leaf($($field: Dim<$ty>),*) -> Self {
                Self { $($field),* }
            }

            /// Convenience constructor for callers (e.g. a retry algorithm) that
            /// already hold concrete dimension values rather than `Dim`s.
            pub fn new($($field: impl Into<$ty>),*) -> Self {
                Self { $($field: Dim::Val($field.into())),* }
            }

            /// The key's dimension segments in persisted order. Generated straight
            /// from the dimension list, so it is the same order `from_db_string`
            /// consumes them in — the two can never disagree by hand.
            fn segments(&self) -> [String; SEGMENT_COUNT] {
                [ $( self.$field.as_segment() ),* ]
            }

            /// Serialize to the persisted `cluster_key`. Generated as the exact
            /// mirror of `from_db_string`: both walk the dimension list in the same
            /// order, so a stored row decodes iff the layout is unchanged.
            pub fn as_db_string(&self) -> String {
                let mut out = String::new();
                out.push_str(RETRY_STATS_KEY_VERSION);
                out.push(KEY_SEPARATOR);
                out.push_str(&self.segments().join(&SEGMENT_SEPARATOR.to_string()));
                out
            }

            /// Parse a persisted `cluster_key`. Generated as the exact mirror of
            /// `as_db_string`. Parsing is strict about arity (exactly
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
                let Ok([$($field),*]) = <[&str; SEGMENT_COUNT]>::try_from(segments) else {
                    logger::error!(
                        cluster_key = raw,
                        expected_segments = SEGMENT_COUNT,
                        "revenue_recovery_retry_stats: cluster key has the wrong number of segments"
                    );
                    return None;
                };
                Some(Self {
                    $(
                        $field: Dim::<$ty>::parse_segment($field)
                            .or_else(|| Self::log_segment_failure(raw, stringify!($field), $field))?,
                    )*
                })
            }
        }
    };
}

// The cluster key's single source of truth.
//
// ⚠️  ORDER IS A PERSISTENCE CONTRACT. The `<index> =>` position of each
//     dimension below is the exact segment position it occupies in every row
//     already stored in the database. Because serialization is positional:
//
//       • REORDERING two dimensions silently swaps the meaning of every stored
//         segment. For same-typed dimensions (e.g. two `String`s) the generated
//         code stays byte-identical and even the roundtrip tests still pass — the
//         corruption is invisible to both the type checker and the tests.
//       • INSERTING a dimension anywhere but the end shifts every following
//         segment by one, so old rows decode into the wrong fields.
//
//     The explicit indices exist to make both mistakes a COMPILE ERROR:
//     `assert_contiguous_indices` requires them to stay `0, 1, 2, …` in written
//     order, so a reorder or a mid-list insert fails the build.
//
//     The ONLY safe change is to APPEND a new dimension with the next index.
//     Never edit the index of an existing dimension.
define_retry_stats_key! {
    /// The dimensional key a retry outcome is recorded under
    pub struct RetryStatsClusterKey {
        0 => error_code: StandardisedCode,
        1 => card_type: CardType,
        2 => issuer: String,
    }
}

impl RetryStatsClusterKey {
    /// Build an error-code-only key: keyed on the error code with `card_type`
    /// and `issuer` left `Unknown`, serializing to `error_code/UNK/UNK`
    pub fn from_error_code(error_code: StandardisedCode) -> Self {
        Self {
            error_code: Dim::Val(error_code),
            card_type: Dim::Unknown,
            issuer: Dim::Unknown,
        }
    }

    /// Redis key of the per-cluster-key lock serializing every writer of this
    /// key's stats document
    pub fn redis_locking_key(&self) -> String {
        format!("revenue_recovery_retry_stats_lock:{}", self.as_db_string())
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
    fn segment_count_matches_declared_dimensions() {
        // Pins the generated arity: this is `error_code / card_type / issuer`.
        // If this changes you added or removed a dimension — a layout change that
        // must bump RETRY_STATS_KEY_VERSION and migrate existing rows.
        assert_eq!(SEGMENT_COUNT, 3);
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
