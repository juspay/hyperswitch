mod connection;
pub mod errors;
pub mod services;
pub mod settings;
mod utils;
use std::sync::Arc;

use storage_models::kv;

use crate::{connection::pg_connection, services::Store};

pub async fn start_drainer(
    store: Arc<Store>,
    number_of_streams: u8,
    max_read_count: u64,
) -> errors::DrainerResult<()> {
    let mut stream_index: u8 = 0;

    loop {
        if utils::is_stream_available(stream_index, store.clone()).await {
            tokio::spawn(drainer_handler(store.clone(), stream_index, max_read_count));
        }
        stream_index = utils::increment_stream_index(stream_index, number_of_streams);
    }
}

async fn drainer_handler(
    store: Arc<Store>,
    stream_index: u8,
    max_read_count: u64,
) -> errors::DrainerResult<()> {
    let stream_name = utils::get_drainer_stream_name(store.clone(), stream_index);
    let drainer_result = drainer(store.clone(), max_read_count, stream_name.as_str()).await;

    if let Err(_e) = drainer_result {
        //TODO: LOG ERRORs
    }

    let flag_stream_name = utils::get_stream_key_flag(store.clone(), stream_index);
    //TODO: USE THE RESULT FOR LOGGING
    utils::make_stream_available(flag_stream_name.as_str(), store.redis_conn.as_ref()).await
}

async fn drainer(
    store: Arc<Store>,
    max_read_count: u64,
    stream_name: &str,
) -> errors::DrainerResult<()> {
    let stream_read =
        utils::read_from_stream(stream_name, max_read_count, store.redis_conn.as_ref()).await?; // this returns the error.

    // parse_stream_entries returns error if no entries is found, handle it
    let (entries, last_entry_id) = utils::parse_stream_entries(&stream_read, stream_name)?;

    let read_count = entries.len();

    // TODO: Handle errors when deserialization fails and when DB error occurs
    for entry in entries {
        let typed_sql = entry.1.get("typed_sql").map_or(String::new(), Clone::clone);
        let result = serde_json::from_str::<kv::DBOperation>(&typed_sql);
        let db_op = match result {
            Ok(f) => f,
            Err(_err) => continue, // TODO: handle error
        };

        let conn = pg_connection(&store.master_pool).await;
        let insert_op = "insert";
        let update_op = "update";
        let payment_intent = "payment_intent";
        let payment_attempt = "payment_attempt";
        match db_op {
            // TODO: Handle errors
            kv::DBOperation::Insert { insertable } => match insertable {
                kv::Insertable::PaymentIntent(a) => {
                    macro_util::handle_resp!(a.insert(&conn).await, insert_op, payment_intent)
                }
                kv::Insertable::PaymentAttempt(a) => {
                    macro_util::handle_resp!(a.insert(&conn).await, insert_op, payment_attempt)
                }
                kv::Insertable::Refund(a) => {
                    macro_util::handle_resp!(a.insert(&conn).await, "ins", "ref")
                }
            },
            kv::DBOperation::Update { updatable } => match updatable {
                kv::Updateable::PaymentIntentUpdate(a) => {
                    macro_util::handle_resp!(
                        a.orig.update(&conn, a.update_data).await,
                        update_op,
                        payment_intent
                    )
                }
                kv::Updateable::PaymentAttemptUpdate(a) => {
                    macro_util::handle_resp!(
                        a.orig.update(&conn, a.update_data).await,
                        update_op,
                        payment_attempt
                    )
                }
                kv::Updateable::RefundUpdate(a) => {
                    macro_util::handle_resp!(a.orig.update(&conn, a.update_data).await, "up", "ref")
                }
            },
            kv::DBOperation::Delete => {
                // TODO: Implement this
                println!("Not implemented!");
            }
        };
    }

    let entries_trimmed =
        utils::trim_from_stream(stream_name, last_entry_id.as_str(), &store.redis_conn).await?;

    if read_count != entries_trimmed {
        // TODO: log
    }

    Ok(())
}

mod macro_util {

    macro_rules! handle_resp {
        ($result:expr,$op_type:expr, $table:expr) => {
            match $result {
                Ok(aa) => println!("Ok|{}|{}|{:?}|", $op_type, $table, aa),
                Err(err) => println!("Err|{}|{}|{:?}|", $op_type, $table, err),
            }
        };
    }
    pub(crate) use handle_resp;
}
