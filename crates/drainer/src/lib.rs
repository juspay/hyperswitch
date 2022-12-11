use errors::DrainerError;
use router::services::Store;
use std::sync::Arc;
pub mod errors;
pub mod utils;
use async_bb8_diesel::{AsyncConnection, ConnectionError, ConnectionManager, PoolError};
use bb8::{CustomizeConnection, PooledConnection};
use utils::*;

pub async fn start_drainer(
    store: &Arc<Store>,
    number_of_streams: &u8,
    max_read_count: &usize,
) -> Result<(), errors::DrainerError> {
    let mut drainer_index: u8 = 1;
    
    tokio::spawn(drainer_handler(
        store.clone(),
        1,
        max_read_count.clone(),
    ));

    loop {
        // if is_stream_available(&drainer_index, store.clone()).await {
        //     println!("start_drainer {}", drainer_index);
        //     tokio::spawn(drainer_handler(
        //         store.clone(),
        //         drainer_index.clone(),
        //         max_read_count.clone(),
        //     ));
        // }
        // increment_drainer_index(&mut drainer_index, &number_of_streams);
        // // if drainer_index == 61 {
        // //     break Ok(());
        // // }
    }
}

async fn drainer_handler(store: Arc<Store>, stream_index: u8, max_read_count: usize) {
    println!(
        "drainer_handler start stream_index{} max_read_count{}",
        stream_index, max_read_count
    );
    let stream_name = store.drainer_stream(stream_index.to_string().as_str());
    let flag_stream_name = format!("{}_in_use", stream_name.as_str());
    let drainer_result = drainer(&store, max_read_count, stream_name.as_str()).await;

    if let Err(_e) = drainer_result {
        //TODO: LOG ERRORs
        // println!("ERROR {:?}",_e);
        if let DrainerError::StreamReadError(e) = _e {
            println!("ERROR drainer_handler {:?}", e);
        }
    }

    //TODO: USE THE RESULT FOR LOGGING
    let p = make_stream_available(flag_stream_name.as_str(), store.redis_conn.as_ref()).await;
    match p {
        Ok(i) => println!("succcess del {}", stream_name),
        Err(e) => println!("delete {:?} {}", e, stream_name),
    }
}

async fn drainer(
    store: &Arc<Store>,
    max_read_count: usize,
    stream_name: &str,
) -> Result<(), DrainerError> {
    let stream_length = get_stream_length(store.redis_conn.as_ref(), stream_name).await?;
    println!("drainer {}, len -> {}", &stream_name, stream_length);
    if stream_length == 0 {
        return Ok(());
    }

    println!("drainer {}, len -> {}", &stream_name, stream_length);

    let read_count = determine_read_count(&stream_length, &max_read_count);
    let stream_read = read_from_stream(&stream_name, read_count, store.redis_conn.as_ref()).await?; // this returns the error.
    println!("drainer stream_read  {:?}", stream_read);
    // parse_stream_entries returns error if no entries is found
    let (entries, last_entry_id) = parse_stream_entries(&stream_read, stream_name)?;
    println!("drainer  {}", entries.len());

    for entry in entries {
        //TODO: PROCESS ENTRIES
        println!("{:#?}", entry);
        let sql = entry
            .1
            .get("sql")
            .ok_or(DrainerError::MessudUp("getting sql from redis".to_owned()))?
            .to_owned();

        let binds = entry
            .1
            .get("binds")
            .ok_or(DrainerError::MessudUp("getting sql from redis".to_owned()))?
            .to_owned();

        let act_binds = serde_json::from_str::<Vec<Option<Vec<u8>>>>(binds.as_str()).unwrap();
        println!("decoding bu=inds: ");
        let decode_binds = act_binds;
        //     .iter()
        //     .map(|bytes| bytes.as_ref().map(|p| hex::decode(p).unwrap()))
        //     .collect::<Vec<_>>();

            println!("going to build rawrawquer decode_binds: {:#?}", decode_binds);
        let o = diesel::query_builder::raw_query::RawRawQuery {
            raw_sql: sql.to_owned(),
            raw_binds: decode_binds,
        };
        println!("built rawarawquery , going to add in db: ");
        store
            .master_pool
            .conn
            .run::<_,PoolError,_>(|conn| Ok(diesel::query_builder::raw_query::execute(o, conn).unwrap()))
            .await
            .unwrap();
        
        println!("succesfully added: ");
    }

    let entries_trimmed =
        trim_from_stream(stream_name, last_entry_id.as_str(), &store.redis_conn).await?;

    if read_count != entries_trimmed {
        // TODO: log
    }

    Ok(())
}
