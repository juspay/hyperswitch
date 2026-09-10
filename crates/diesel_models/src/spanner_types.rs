//! SQL type aliases that let one `schema.rs` target both PostgreSQL and Spanner.
//!
//! Spanner's PostgreSQL dialect differs from Postgres in four places that touch
//! this schema. Verified against the emulator via PGAdapter 0.55.3:
//!
//! | Postgres                      | Spanner            | Columns |
//! |-------------------------------|--------------------|---------|
//! | `timestamp without time zone` | rejected outright  | 103     |
//! | `json`                        | rejected, use jsonb| 17      |
//! | `CREATE TYPE ... AS ENUM`     | rejected           | 46      |
//!
//! The enum case is handled in `router_derive` (the marker's OID becomes `text`).
//! The other three are handled here.
//!
//! The trick is that `diesel.toml` emits these as *explicit* imports into every
//! generated table block, and an explicit import shadows the `diesel::sql_types::*`
//! glob. So `schema.rs` keeps saying `created_at -> Timestamp` and the models keep
//! their `PrimitiveDateTime`; only the wire type underneath changes.

#[cfg(not(feature = "spanner"))]
mod inner {
    pub use diesel::sql_types::{Json as HsJson, Timestamp as HsTimestamp};
}

#[cfg(feature = "spanner")]
mod inner {
    //! Spanner equivalents. Both are plain aliases to types diesel already
    //! supports end to end.
    //!
    //! An earlier attempt declared custom marker structs carrying Spanner's OIDs
    //! so that `i32`/`i16` fields could stay narrow. That does not work: the
    //! orphan rule rejects `impl AsExpression<Nullable<HsInt4>> for i32` (both
    //! `Nullable` and `i32` are foreign), and `table!` additionally demands the
    //! numeric `ops::{Add, Sub, Mul, Div}` traits. So the 19 narrow integer
    //! columns were widened to bigint in Postgres too (see the
    //! 2026-09-10-180000 migration) and the model fields are plain `i64` -
    //! one schema shape for both backends, and no integer shim at all.

    /// 103 timestamp columns. Diesel already implements `ToSql`/`FromSql<
    /// Timestamptz, Pg>` for `PrimitiveDateTime` with the identical wire
    /// encoding, so no model field changes.
    pub use diesel::sql_types::Timestamptz as HsTimestamp;

    /// 17 `json` columns. `serde_json::Value` implements both; newtypes
    /// annotated `#[diesel(sql_type = Json)]` must switch to this alias.
    pub use diesel::sql_types::Jsonb as HsJson;
}

pub use inner::{HsJson, HsTimestamp};

// Compile-time guard on what the aliases actually resolve to. A closure coerces
// to these fn-pointer types only if the alias is the type it claims to be, so
// re-pointing one by mistake is a compile error here rather than a confusing
// trait failure in schema.rs.
//
// Note this does NOT catch the module failing to parse: if `inner` breaks, the
// explicit import in schema.rs fails and the `diesel::sql_types::*` glob
// silently supplies the ORIGINAL type. Check for `error: expected item` (no
// E-code, so `grep 'error\['` hides it) when types resolve unexpectedly.
#[cfg(feature = "spanner")]
const _: () = {
    const _JSON_IS_JSONB: fn(HsJson) -> diesel::sql_types::Jsonb = |t| t;
    const _TS_IS_TIMESTAMPTZ: fn(HsTimestamp) -> diesel::sql_types::Timestamptz = |t| t;
};

#[cfg(not(feature = "spanner"))]
const _: () = {
    const _JSON_IS_JSON: fn(HsJson) -> diesel::sql_types::Json = |t| t;
    const _TS_IS_TIMESTAMP: fn(HsTimestamp) -> diesel::sql_types::Timestamp = |t| t;
};
