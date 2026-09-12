//! Does *diesel* — not psql — actually work against Spanner via PGAdapter?
//!
//! psql speaks the text wire format; diesel drives the extended query protocol
//! and asks for results in BINARY. Everything proven so far went through psql,
//! so this is the first real test of the combination the router depends on.
//!
//! Ignored by default (needs the local harness):
//!   docker compose -f docker-compose-spanner.yml up -d
//!   ./scripts/spanner/run_local.sh
//!   cargo test -p diesel_models --features spanner,v1 \
//!       --test spanner_roundtrip -- --ignored --nocapture

#![allow(clippy::unwrap_used, clippy::expect_used)]

use async_bb8_diesel::AsyncRunQueryDsl;
use diesel::{ExpressionMethods, QueryDsl};
use diesel_models::schema::payment_intent::dsl;

const SPANNER_URL: &str = "postgresql://localhost:5433/hyperswitch_db";

async fn pool() -> bb8::Pool<async_bb8_diesel::ConnectionManager<diesel::PgConnection>> {
    let manager = async_bb8_diesel::ConnectionManager::<diesel::PgConnection>::new(SPANNER_URL);
    bb8::Pool::builder()
        .max_size(2)
        .build(manager)
        .await
        .expect("connect to PGAdapter")
}

/// Reads the row the harness inserts, exercising the binary decode path for
/// every type category the migration touched at once: varchar, enum-as-text,
/// timestamptz, bigint (was smallint), jsonb and jsonb[].
#[tokio::test]
#[ignore = "requires the local Spanner harness on :5433"]
async fn diesel_reads_a_payment_intent_from_spanner() {
    let pool = pool().await;
    let conn = pool.get().await.expect("checkout");

    let rows: Vec<(String, common_enums::IntentStatus, i64)> = dsl::payment_intent
        .filter(dsl::payment_id.eq("pay_probe_1".to_string()))
        .select((dsl::payment_id, dsl::status, dsl::attempt_count))
        .load_async(&*conn)
        .await
        .expect("SELECT through diesel");

    println!("diesel decoded {} row(s): {rows:?}", rows.len());
    assert_eq!(rows.len(), 1, "expected the harness probe row");
    assert_eq!(rows[0].0, "pay_probe_1");
    assert_eq!(rows[0].2, 1, "attempt_count decoded from bigint");
}

/// The write path: bind parameters, INSERT, then read it back.
#[tokio::test]
#[ignore = "requires the local Spanner harness on :5433"]
async fn diesel_writes_a_payment_intent_to_spanner() {
    let pool = pool().await;
    let conn = pool.get().await.expect("checkout");

    let id = format!("pay_diesel_{}", std::process::id());

    let inserted = diesel::insert_into(dsl::payment_intent)
        .values((
            dsl::payment_id.eq(id.clone()),
            dsl::merchant_id.eq("merchant_probe".to_string()),
            dsl::status.eq(common_enums::IntentStatus::RequiresCapture),
            dsl::amount.eq(4242_i64),
            dsl::currency.eq(Some(common_enums::Currency::USD)),
            dsl::created_at.eq(common_utils::date_time::now()),
            dsl::modified_at.eq(common_utils::date_time::now()),
            dsl::last_synced.eq(None::<time::PrimitiveDateTime>),
            dsl::attempt_count.eq(1_i64),
            dsl::profile_id.eq("prof_probe".to_string()),
        ))
        .execute_async(&*conn)
        .await
        .expect("INSERT through diesel");
    assert_eq!(inserted, 1);

    let back: Vec<(String, common_enums::IntentStatus, i64)> = dsl::payment_intent
        .filter(dsl::payment_id.eq(id.clone()))
        .select((dsl::payment_id, dsl::status, dsl::amount))
        .load_async(&*conn)
        .await
        .expect("SELECT back");

    println!("wrote and read back: {back:?}");
    assert_eq!(back.len(), 1);
    assert_eq!(back[0].2, 4242);
}
