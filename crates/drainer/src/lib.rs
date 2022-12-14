pub mod errors;
pub mod utils;
use self::errors::DrainerError;
use router::{connection::pg_connection, db::kv_gen, services::Store};
use std::sync::Arc;

pub async fn start_drainer(
    store: Arc<Store>,
    number_of_streams: u8,
    max_read_count: u64,
) -> Result<(), errors::DrainerError> {
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
) -> Result<(), DrainerError> {
    let stream_name = store.drainer_stream(stream_index.to_string().as_str());

    let drainer_result = drainer(store.clone(), max_read_count, stream_name.as_str()).await;

    if let Err(_e) = drainer_result {
        //TODO: LOG ERRORs
    }

    let flag_stream_name = utils::get_steam_key_flag(store.clone(), stream_index.to_string());
    //TODO: USE THE RESULT FOR LOGGING
    utils::make_stream_available(flag_stream_name.as_str(), store.redis_conn.as_ref()).await
}

async fn drainer(
    store: Arc<Store>,
    max_read_count: u64,
    stream_name: &str,
) -> Result<(), DrainerError> {
    let stream_read =
        utils::read_from_stream(stream_name, max_read_count, store.redis_conn.as_ref()).await?; // this returns the error.

    // parse_stream_entries returns error if no entries is found, handle it
    let (entries, last_entry_id) = utils::parse_stream_entries(&stream_read, stream_name)?;

    let read_count = entries.len();

    // TODO: Handle errors when deserialization fails and when DB error occurs
    for entry in entries {
        let typed_sql = entry.1.get("typed_sql").map_or(String::new(), Clone::clone);
        let result = serde_json::from_str::<kv_gen::DBOperation>(&typed_sql);
        let Ok(db_op) = result else {
            continue; // TODO: handle error
        };

        let conn = pg_connection(&store.master_pool).await;

        match db_op {
            // TODO: Handle errors
            kv_gen::DBOperation::Insert(a) => match a.insertable {
                kv_gen::Insertables::PaymentIntent(a) => {
                    macro_util::handle_resp!(a.insert(&conn).await, "ins", "pi")
                }
                kv_gen::Insertables::PaymentAttempt(a) => {
                    macro_util::handle_resp!(a.insert(&conn).await, "ins", "pa")
                }
            },
            kv_gen::DBOperation::Update(a) => match a.updateable {
                kv_gen::Updateables::PaymentIntentUpdate(a) => {
                    macro_util::handle_resp!(a.orig.update(&conn, a.update_data).await, "up", "pi")
                }
                kv_gen::Updateables::PaymentAttemptUpdate(a) => {
                    macro_util::handle_resp!(a.orig.update(&conn, a.update_data).await, "up", "pa")
                }
            },
            kv_gen::DBOperation::Delete => todo!(),
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
