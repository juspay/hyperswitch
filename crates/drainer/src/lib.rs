mod connection;
pub mod errors;
pub mod logger;
pub(crate) mod metrics;
pub mod services;
pub mod settings;
mod utils;
use std::sync::{atomic, Arc};

use common_utils::signals::get_allowed_signals;
use diesel_models::kv;
use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};
use tokio::sync::{mpsc, oneshot};

use crate::{connection::pg_connection, services::Store};

pub async fn start_drainer(
    store: Arc<Store>,
    number_of_streams: u8,
    max_read_count: u64,
    shutdown_interval: u32,
    loop_interval: u32,
) -> errors::DrainerResult<()> {
    let mut stream_index: u8 = 0;
    let mut jobs_picked: u8 = 0;

    let mut shutdown_interval =
        tokio::time::interval(std::time::Duration::from_millis(shutdown_interval.into()));
    let mut loop_interval =
        tokio::time::interval(std::time::Duration::from_millis(loop_interval.into()));

    let signal =
        get_allowed_signals()
            .into_report()
            .change_context(errors::DrainerError::SignalError(
                "Failed while getting allowed signals".to_string(),
            ))?;

    let (redis_error_tx, redis_error_rx) = oneshot::channel();
    let (tx, mut rx) = mpsc::channel(1);
    let handle = signal.handle();
    let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, tx.clone()));

    let redis_conn_clone = store.redis_conn.clone();

    // Spawn a task to monitor if redis is down or not
    tokio::spawn(async move { redis_conn_clone.on_error(redis_error_tx).await });

    //Spawns a task to send shutdown signal if redis goes down
    tokio::spawn(redis_error_receiver(redis_error_rx, tx));

    let active_tasks = Arc::new(atomic::AtomicU64::new(0));
    'event: loop {
        metrics::DRAINER_HEALTH.add(&metrics::CONTEXT, 1, &[]);
        match rx.try_recv() {
            Err(mpsc::error::TryRecvError::Empty) => {
                if utils::is_stream_available(stream_index, store.clone()).await {
                    tokio::spawn(drainer_handler(
                        store.clone(),
                        stream_index,
                        max_read_count,
                        active_tasks.clone(),
                    ));
                    jobs_picked += 1;
                }
                (stream_index, jobs_picked) = utils::increment_stream_index(
                    (stream_index, jobs_picked),
                    number_of_streams,
                    &mut loop_interval,
                )
                .await;
            }
            Ok(()) | Err(mpsc::error::TryRecvError::Disconnected) => {
                logger::info!("Awaiting shutdown!");
                metrics::SHUTDOWN_SIGNAL_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
                let shutdown_started = tokio::time::Instant::now();
                rx.close();
                loop {
                    if active_tasks.load(atomic::Ordering::Acquire) == 0 {
                        logger::info!("Terminating drainer");
                        metrics::SUCCESSFUL_SHUTDOWN.add(&metrics::CONTEXT, 1, &[]);
                        let shutdown_ended = shutdown_started.elapsed().as_secs_f64() * 1000f64;
                        metrics::CLEANUP_TIME.record(&metrics::CONTEXT, shutdown_ended, &[]);
                        break 'event;
                    }
                    shutdown_interval.tick().await;
                }
            }
        }
    }
    handle.close();
    task_handle
        .await
        .into_report()
        .change_context(errors::DrainerError::UnexpectedError(
            "Failed while joining signal handler".to_string(),
        ))?;

    Ok(())
}

pub async fn redis_error_receiver(rx: oneshot::Receiver<()>, shutdown_channel: mpsc::Sender<()>) {
    match rx.await {
        Ok(_) => {
            logger::error!("The redis server failed ");
            let _ = shutdown_channel.send(()).await.map_err(|err| {
                logger::error!("Failed to send signal to the shutdown channel {err}")
            });
        }
        Err(err) => {
            logger::error!("Channel receiver error{err}");
        }
    }
}

async fn drainer_handler(
    store: Arc<Store>,
    stream_index: u8,
    max_read_count: u64,
    active_tasks: Arc<atomic::AtomicU64>,
) -> errors::DrainerResult<()> {
    active_tasks.fetch_add(1, atomic::Ordering::Release);

    let stream_name = utils::get_drainer_stream_name(store.clone(), stream_index);

    let drainer_result =
        Box::pin(drainer(store.clone(), max_read_count, stream_name.as_str())).await;

    if let Err(error) = drainer_result {
        logger::error!(?error)
    }

    let flag_stream_name = utils::get_stream_key_flag(store.clone(), stream_index);

    //TODO: USE THE RESULT FOR LOGGING
    let output =
        utils::make_stream_available(flag_stream_name.as_str(), store.redis_conn.as_ref()).await;
    active_tasks.fetch_sub(1, atomic::Ordering::Release);
    output
}

#[instrument(skip_all, fields(global_id, request_id, session_id))]
async fn drainer(
    store: Arc<Store>,
    max_read_count: u64,
    stream_name: &str,
) -> errors::DrainerResult<()> {
    let stream_read =
        match utils::read_from_stream(stream_name, max_read_count, store.redis_conn.as_ref()).await
        {
            Ok(result) => result,
            Err(error) => {
                if let errors::DrainerError::RedisError(redis_err) = error.current_context() {
                    if let redis_interface::errors::RedisError::StreamEmptyOrNotAvailable =
                        redis_err.current_context()
                    {
                        metrics::STREAM_EMPTY.add(&metrics::CONTEXT, 1, &[]);
                        return Ok(());
                    } else {
                        return Err(error);
                    }
                } else {
                    return Err(error);
                }
            }
        };
    // parse_stream_entries returns error if no entries is found, handle it
    let (entries, last_entry_id) = utils::parse_stream_entries(&stream_read, stream_name)?;
    let read_count = entries.len();

    metrics::JOBS_PICKED_PER_STREAM.add(
        &metrics::CONTEXT,
        u64::try_from(read_count).unwrap_or(u64::MIN),
        &[metrics::KeyValue {
            key: "stream".into(),
            value: stream_name.to_string().into(),
        }],
    );

    let session_id = common_utils::generate_id_with_default_len("drainer_session");

    // TODO: Handle errors when deserialization fails and when DB error occurs
    for entry in entries {
        let typed_sql = entry.1.get("typed_sql").map_or(String::new(), Clone::clone);
        let request_id = entry
            .1
            .get("request_id")
            .map_or(String::new(), Clone::clone);
        let global_id = entry.1.get("global_id").map_or(String::new(), Clone::clone);

        tracing::Span::current().record("request_id", request_id);
        tracing::Span::current().record("global_id", global_id);
        tracing::Span::current().record("session_id", &session_id);

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
        let refund = "refund";
        let reverse_lookup = "reverse_lookup";
        let connector_response = "connector_response";
        let address = "address";
        match db_op {
            // TODO: Handle errors
            kv::DBOperation::Insert { insertable } => {
                let (_, execution_time) = common_utils::date_time::time_it(|| async {
                    match insertable {
                        kv::Insertable::PaymentIntent(a) => {
                            macro_util::handle_resp!(
                                a.insert(&conn).await,
                                insert_op,
                                payment_intent
                            )
                        }
                        kv::Insertable::PaymentAttempt(a) => {
                            macro_util::handle_resp!(
                                a.insert(&conn).await,
                                insert_op,
                                payment_attempt
                            )
                        }
                        kv::Insertable::Refund(a) => {
                            macro_util::handle_resp!(a.insert(&conn).await, insert_op, refund)
                        }
                        kv::Insertable::ConnectorResponse(a) => {
                            macro_util::handle_resp!(
                                a.insert(&conn).await,
                                insert_op,
                                connector_response
                            )
                        }
                        kv::Insertable::Address(addr) => {
                            macro_util::handle_resp!(addr.insert(&conn).await, insert_op, address)
                        }
                        kv::Insertable::ReverseLookUp(rev) => {
                            macro_util::handle_resp!(
                                rev.insert(&conn).await,
                                insert_op,
                                reverse_lookup
                            )
                        }
                    }
                })
                .await;
                metrics::QUERY_EXECUTION_TIME.record(
                    &metrics::CONTEXT,
                    execution_time,
                    &[metrics::KeyValue {
                        key: "operation".into(),
                        value: insert_op.into(),
                    }],
                );
            }
            kv::DBOperation::Update { updatable } => {
                let (_, execution_time) = common_utils::date_time::time_it(|| async {
                    match updatable {
                        kv::Updateable::PaymentIntentUpdate(a) => {
                            macro_util::handle_resp!(
                                a.orig.update(&conn, a.update_data).await,
                                update_op,
                                payment_intent
                            )
                        }
                        kv::Updateable::PaymentAttemptUpdate(a) => {
                            macro_util::handle_resp!(
                                a.orig.update_with_attempt_id(&conn, a.update_data).await,
                                update_op,
                                payment_attempt
                            )
                        }
                        kv::Updateable::RefundUpdate(a) => {
                            macro_util::handle_resp!(
                                a.orig.update(&conn, a.update_data).await,
                                update_op,
                                refund
                            )
                        }
                        kv::Updateable::ConnectorResponseUpdate(a) => macro_util::handle_resp!(
                            a.orig.update(&conn, a.update_data).await,
                            update_op,
                            connector_response
                        ),
                        kv::Updateable::AddressUpdate(a) => macro_util::handle_resp!(
                            a.orig.update(&conn, a.update_data).await,
                            update_op,
                            address
                        ),
                    }
                })
                .await;
                metrics::QUERY_EXECUTION_TIME.record(
                    &metrics::CONTEXT,
                    execution_time,
                    &[metrics::KeyValue {
                        key: "operation".into(),
                        value: update_op.into(),
                    }],
                );
            }
            kv::DBOperation::Delete => {
                // [#224]: Implement this
                logger::error!("Not implemented!");
            }
        };
    }

    let entries_trimmed =
        utils::trim_from_stream(stream_name, last_entry_id.as_str(), &store.redis_conn).await?;

    if read_count != entries_trimmed {
        logger::error!(
            read_entries = %read_count,
            trimmed_entries = %entries_trimmed,
            ?entries,
            "Assertion Failed no. of entries read from the stream doesn't match no. of entries trimmed"
        );
    }

    Ok(())
}

mod macro_util {

    macro_rules! handle_resp {
        ($result:expr,$op_type:expr, $table:expr) => {
            match $result {
                Ok(inner_result) => {
                    logger::info!(operation = %$op_type, table = %$table, ?inner_result);
                    metrics::SUCCESSFUL_QUERY_EXECUTION.add(&metrics::CONTEXT, 1, &[
                        metrics::KeyValue {
                            key: "operation".into(),
                            value: $table.into(),
                        }
                    ]);
                }
                Err(err) => {
                    logger::error!(operation = %$op_type, table = %$table, ?err);
                    metrics::ERRORS_WHILE_QUERY_EXECUTION.add(&metrics::CONTEXT, 1, &[
                        metrics::KeyValue {
                            key: "operation".into(),
                            value: $table.into(),
                        }
                    ]);
                }
            }
        };
    }
    pub(crate) use handle_resp;
}
