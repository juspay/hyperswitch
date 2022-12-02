use errors::DrainerError;
use router::services::Store;
use std::sync::Arc;

pub mod errors;
pub mod utils;

use utils::*;

pub async fn start_drainer(
    store: &Arc<Store>,
    number_of_streams: &u8,
    max_read_count: &usize,
) -> Result<(), errors::DrainerError> {
    let mut drainer_index: u8 = 0;

    loop {
        if is_stream_available(&drainer_index, store.clone()).await {
            tokio::spawn(drainer_handler(
                store.clone(),
                drainer_index.clone(),
                max_read_count.clone(),
            ));
        }
        increment_drainer_index(&mut drainer_index, &number_of_streams);
    }
}

async fn drainer_handler(store: Arc<Store>, stream_index: u8, max_read_count: usize) {
    let stream_name = store.drainer_stream(stream_index.to_string().as_str());
    let drainer_result = drainer(&store, max_read_count, stream_name.as_str()).await;

    if let Err(_e) = drainer_result {
        //TODO: LOG ERRORs
        println!("ERROR {:?}",_e);
    }

    //TODO: USE THE RESULT FOR LOGGING
    let _ = make_stream_available(stream_name.as_str(), store.redis_conn.as_ref()).await;
}

async fn drainer(
    store: &Arc<Store>,
    max_read_count: usize,
    stream_name: &str,
) -> Result<(), DrainerError> {
    let stream_length = get_stream_length(store.redis_conn.as_ref(), stream_name).await?;

    if stream_length == 0 {
        return Ok(());
    }

    let read_count = determine_read_count(&stream_length, &max_read_count);
    let stream_read = read_from_stream(&stream_name, read_count, store.redis_conn.as_ref()).await?;

    // parse_stream_entries returns error if no entries is found
    let (entries, last_entry_id) = parse_stream_entries(&stream_read, stream_name)?;

    for _entry in entries {
        //TODO: PROCESS ENTRIES
    }

    let entries_trimmed =
        trim_from_stream(stream_name, last_entry_id.as_str(), &store.redis_conn).await?;

    if read_count != entries_trimmed {
        // TODO: log
    }

    Ok(())
}
