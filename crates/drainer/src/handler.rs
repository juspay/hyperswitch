use std::sync::{atomic, Arc};

use common_utils::signals::get_allowed_signals;
use error_stack::{IntoReport, ResultExt};
use tokio::{
    sync::{mpsc, oneshot},
    time::Interval,
};

use crate::{
    errors, instrument, logger, metrics, query::ExecuteQuery, tracing, utils, DrainerSettings,
    Store, StreamData,
};

type MpscChannel<T> = (mpsc::Sender<T>, mpsc::Receiver<T>);

pub struct Handler {
    mpsc_channel: MpscChannel<()>,
    shutdown_interval: Interval,
    loop_interval: Interval,
    active_tasks: Arc<atomic::AtomicU64>,
    conf: DrainerSettings,
    store: Arc<Store>,
}

impl Handler {
    pub fn from_conf(conf: DrainerSettings, store: Arc<Store>) -> Self {
        let shutdown_interval = tokio::time::interval(std::time::Duration::from_millis(
            conf.shutdown_interval.into(),
        ));
        let loop_interval =
            tokio::time::interval(std::time::Duration::from_millis(conf.loop_interval.into()));

        let active_tasks = Arc::new(atomic::AtomicU64::new(0));

        let (tx, rx) = mpsc::channel(1);

        Self {
            mpsc_channel: (tx, rx),
            shutdown_interval,
            loop_interval,
            active_tasks,
            conf,
            store,
        }
    }

    pub async fn spawn(&mut self) -> errors::DrainerResult<()> {
        let mut stream_index: u8 = 0;
        let jobs_picked = Arc::new(atomic::AtomicU8::new(0));

        let tx = self.mpsc_channel.0.clone();
        let rx = &mut self.mpsc_channel.1;

        let signal = get_allowed_signals().into_report().change_context(
            errors::DrainerError::SignalError("Failed while getting allowed signals".to_string()),
        )?;
        let handle = signal.handle();
        let task_handle = tokio::spawn(common_utils::signals::signal_handler(signal, tx.clone()));

        'event: loop {
            metrics::DRAINER_HEALTH.add(&metrics::CONTEXT, 1, &[]);
            match rx.try_recv() {
                Err(mpsc::error::TryRecvError::Empty) => {
                    if self.store.is_stream_available(stream_index).await {
                        tokio::spawn(drainer_handler(
                            self.store.clone(),
                            stream_index,
                            self.conf.max_read_count,
                            self.active_tasks.clone(),
                            jobs_picked.clone(),
                        ));
                    }
                    stream_index = utils::increment_stream_index(
                        (stream_index, jobs_picked.clone()),
                        self.store.config.drainer_num_partitions,
                    )
                    .await;
                    self.loop_interval.tick().await;
                }
                Ok(()) | Err(mpsc::error::TryRecvError::Disconnected) => {
                    logger::info!("Awaiting shutdown!");
                    metrics::SHUTDOWN_SIGNAL_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
                    let shutdown_started = tokio::time::Instant::now();
                    rx.close();
                    loop {
                        if self.active_tasks.load(atomic::Ordering::Acquire) == 0 {
                            logger::info!("Terminating drainer");
                            metrics::SUCCESSFUL_SHUTDOWN.add(&metrics::CONTEXT, 1, &[]);
                            let shutdown_ended = shutdown_started.elapsed().as_secs_f64() * 1000f64;
                            metrics::CLEANUP_TIME.record(&metrics::CONTEXT, shutdown_ended, &[]);
                            break 'event;
                        }
                        self.shutdown_interval.tick().await;
                    }
                }
            }
        }

        handle.close();
        let _ = task_handle
            .await
            .map_err(|err| logger::error!("Failed while joining signal handler: {:?}", err));

        Ok(())
    }

    pub fn spawn_error_handlers(&self) -> errors::DrainerResult<()> {
        let tx = self.mpsc_channel.0.clone();
        let (redis_error_tx, redis_error_rx) = oneshot::channel();

        let redis_conn_clone = self.store.redis_conn.clone();

        // Spawn a task to monitor if redis is down or not
        tokio::spawn(async move { redis_conn_clone.on_error(redis_error_tx).await });

        //Spawns a task to send shutdown signal if redis goes down
        tokio::spawn(redis_error_receiver(redis_error_rx, tx));

        Ok(())
    }
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

#[router_env::instrument(skip_all)]
async fn drainer_handler(
    store: Arc<Store>,
    stream_index: u8,
    max_read_count: u64,
    active_tasks: Arc<atomic::AtomicU64>,
    jobs_picked: Arc<atomic::AtomicU8>,
) -> errors::DrainerResult<()> {
    active_tasks.fetch_add(1, atomic::Ordering::Release);

    let stream_name = store.get_drainer_stream_name(stream_index);

    let drainer_result = Box::pin(drainer(
        store.clone(),
        max_read_count,
        stream_name.as_str(),
        jobs_picked,
    ))
    .await;

    if let Err(error) = drainer_result {
        logger::error!(?error)
    }

    let flag_stream_name = store.get_stream_key_flag(stream_index);

    //TODO: USE THE RESULT FOR LOGGING
    let output = store.make_stream_available(flag_stream_name.as_str()).await;
    active_tasks.fetch_sub(1, atomic::Ordering::Release);
    output
}

#[instrument(skip_all, fields(global_id, request_id, session_id))]
async fn drainer(
    store: Arc<Store>,
    max_read_count: u64,
    stream_name: &str,
    jobs_picked: Arc<atomic::AtomicU8>,
) -> errors::DrainerResult<()> {
    let stream_read = match store.read_from_stream(stream_name, max_read_count).await {
        Ok(result) => {
            jobs_picked.fetch_add(1, atomic::Ordering::SeqCst);
            result
        }
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
    let entries = utils::parse_stream_entries(&stream_read, stream_name)?;
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

    let mut last_processed_id = String::new();

    for (entry_id, entry) in entries.clone() {
        let data = match StreamData::from_hashmap(entry) {
            Ok(data) => data,
            Err(err) => {
                logger::error!(operation = "deserialization", err=?err);
                metrics::STREAM_PARSE_FAIL.add(
                    &metrics::CONTEXT,
                    1,
                    &[metrics::KeyValue {
                        key: "operation".into(),
                        value: "deserialization".into(),
                    }],
                );

                // break from the loop in case of a deser error
                break;
            }
        };

        tracing::Span::current().record("request_id", data.request_id);
        tracing::Span::current().record("global_id", data.global_id);
        tracing::Span::current().record("session_id", &session_id);

        match data.typed_sql.execute_query(&store, data.pushed_at).await {
            Ok(_) => {
                last_processed_id = entry_id;
            }
            Err(err) => match err.current_context() {
                // In case of Uniqueviolation we can't really do anything to fix it so just clear
                // it from the stream
                diesel_models::errors::DatabaseError::UniqueViolation => {
                    last_processed_id = entry_id;
                }
                // break from the loop in case of an error in query
                _ => break,
            },
        }
    }

    let entries_trimmed = store
        .trim_from_stream(stream_name, &last_processed_id)
        .await?;

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
