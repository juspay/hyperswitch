use std::sync::Arc;

use errors::DrainerError;
use router::{connection::pg_connection, db::kv_gen::DBOperation, services::Store};
pub mod errors;
pub mod utils;

use utils::*;

pub async fn start_drainer(
    store: Arc<Store>,
    number_of_streams: u8,
    max_read_count: usize,
) -> Result<(), errors::DrainerError> {
    let mut stream_index: u8 = 0;
    tokio::spawn(drainer_handler(store.clone(), 52, max_read_count));
    loop {
        // if is_stream_available(stream_index, store.clone()).await {
        //     println!("start_drainer {}", stream_index);

        // }
        // stream_index = increment_stream_index(stream_index, number_of_streams);
    }
}

async fn drainer_handler(
    store: Arc<Store>,
    stream_index: u8,
    max_read_count: usize,
) -> Result<(), DrainerError> {
    println!(
        "drainer_handler start stream_index{} max_read_count{}",
        stream_index, max_read_count
    );

    let stream_name = store.drainer_stream(stream_index.to_string().as_str());

    drainer(store.clone(), max_read_count, stream_name.as_str()).await?;

    // if let Err(e) = drainer_result {
    //     //TODO: LOG ERRORs
    //     // println!("ERROR {:?}",_e);
    //     println!("ERROR drainer_handler {:?}", e);
    // }

    let flag_stream_name = get_steam_key_flag(store.clone(), stream_index.to_string());
    //TODO: USE THE RESULT FOR LOGGING
    make_stream_available(flag_stream_name.as_str(), store.redis_conn.as_ref()).await
}

async fn drainer(
    store: Arc<Store>,
    max_read_count: usize,
    stream_name: &str,
) -> Result<(), DrainerError> {
    let stream_length = get_stream_length(store.redis_conn.as_ref(), stream_name).await?; // y to get stream length, directly get stream right
    println!("drainer {}, len -> {}", &stream_name, stream_length);
    if stream_length == 0 {
        return Ok(());
    }

    println!("drainer {}, len -> {}", &stream_name, stream_length);

    let read_count = determine_read_count(stream_length, max_read_count);
    let stream_read = read_from_stream(&stream_name, read_count, store.redis_conn.as_ref()).await?; // this returns the error.
    println!("drainer stream_read  {:?}", stream_read);
    // parse_stream_entries returns error if no entries is found
    let (entries, last_entry_id) = parse_stream_entries(&stream_read, stream_name)?;
    println!("drainer  entries {:?}", entries);

    for entry in entries {
        //TODO: PROCESS ENTRIES
        println!("{:#?}", entry);
        let typedsql = entry.1.get("typedsql").unwrap_or(&"".to_owned()).to_owned();
        let f = serde_json::from_str::<DBOperation>(&typedsql);
        let dbop = match f {
            Ok(f) => f,
            Err(err) => continue,
        };

        let conn = pg_connection(&store.master_pool).await;
        let r = match dbop {
            DBOperation::Insert(a) => match a.insertable {
                router::db::kv_gen::Insertables::PaymentIntent(a) => {
                    let p = a.insert(&conn).await;
                    match p {
                        Ok(aa) => println!("succesfully inserted {:#?}", aa),
                        Err(err) => println!("Err not able to insert {:?}", err),
                    }
                }
                router::db::kv_gen::Insertables::PaymentAttempt(a) => {
                    let p = a.insert(&conn).await;
                    match p {
                        Ok(aa) => println!("succesfully inserted {:#?}", aa),
                        Err(err) => println!("Err not able to insert {:?}", err),
                    }
                }
            },
            DBOperation::Update(a) => match a.updateable {
                router::db::kv_gen::Updateables::PaymentIntentUpdate(a) => {
                    let p = a.orig.update(&conn, a.update_data).await;
                    match p {
                        Ok(aa) => println!("succesfully inserted {:#?}", aa),
                        Err(err) => println!("Err not able to insert {:?}", err),
                    }
                }
                router::db::kv_gen::Updateables::PaymentAttemptUpdate(a) => {
                    let p = a.orig.update(&conn, a.update_data).await;
                    match p {
                        Ok(aa) => println!("succesfully inserted {:#?}", aa),
                        Err(err) => println!("Err not able to insert {:?}", err),
                    }
                }
            },
            DBOperation::Delete => todo!(),
        };
        println!("succesfully added: ");
    }

    let entries_trimmed =
        trim_from_stream(stream_name, last_entry_id.as_str(), &store.redis_conn).await?;

    if read_count != entries_trimmed {
        // TODO: log
    }

    Ok(())
}
