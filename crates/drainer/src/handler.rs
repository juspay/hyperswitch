use std::sync::{atomic, Arc};

use tokio::{
    sync::{mpsc, oneshot},
    time::{self, Duration},
};

use crate::{
    errors, instrument, logger, metrics,
    stream::{DrainStream, StreamOperation},
    tracing, utils, DrainerSettings,
};

/// Handler handles the spawning and closing of drainer
/// Arc is used to enable creating a listener for graceful shutdown
#[derive(Clone)]
pub struct Handler<T: StreamOperation + DrainStream + Clone + Send> {
    inner: Arc<HandlerInner<T>>,
}

impl<T: StreamOperation + Clone + Send + DrainStream> std::ops::Deref for Handler<T> {
    type Target = HandlerInner<T>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

pub struct HandlerInner<T: StreamOperation + DrainStream + Clone + Send> {
    shutdown_interval: Duration,
    loop_interval: Duration,
    active_tasks: Arc<atomic::AtomicU64>,
    conf: DrainerSettings,
    stream: T,
    running: Arc<atomic::AtomicBool>,
}

impl<T: StreamOperation + DrainStream + Clone + Send + Sync + 'static> Handler<T> {
    pub fn from_conf(conf: DrainerSettings, stream: T) -> Self {
        let shutdown_interval = Duration::from_millis(conf.shutdown_interval.into());
        let loop_interval = Duration::from_millis(conf.loop_interval.into());

        let active_tasks = Arc::new(atomic::AtomicU64::new(0));

        let running = Arc::new(atomic::AtomicBool::new(true));

        let handler = HandlerInner {
            shutdown_interval,
            loop_interval,
            active_tasks,
            conf,
            stream,
            running,
        };

        Self {
            inner: Arc::new(handler),
        }
    }

    pub fn close(&self) {
        self.running.store(false, atomic::Ordering::SeqCst);
    }

    pub async fn spawn(&self) -> errors::DrainerResult<()> {
        let mut stream_index: u8 = 0;
        let jobs_picked = Arc::new(atomic::AtomicU8::new(0));

        while self.running.load(atomic::Ordering::SeqCst) {
            metrics::DRAINER_HEALTH.add(&metrics::CONTEXT, 1, &[]);
            if self.stream.is_stream_available(stream_index).await {
                tokio::spawn(drainer_handler(
                    self.stream.clone(),
                    stream_index,
                    self.conf.max_read_count,
                    self.active_tasks.clone(),
                    jobs_picked.clone(),
                ));
            }
            stream_index = utils::increment_stream_index(
                (stream_index, jobs_picked.clone()),
                self.stream.store().config.drainer_num_partitions,
            )
            .await;
            time::sleep(self.loop_interval).await;
        }

        Ok(())
    }

    pub(crate) async fn shutdown_listener(&self, mut rx: mpsc::Receiver<()>) {
        while let Some(_c) = rx.recv().await {
            logger::info!("Awaiting shutdown!");
            metrics::SHUTDOWN_SIGNAL_RECEIVED.add(&metrics::CONTEXT, 1, &[]);
            let shutdown_started = tokio::time::Instant::now();
            rx.close();

            //Check until the active tasks are zero. This does not include the tasks in the stream.
            while self.active_tasks.load(atomic::Ordering::SeqCst) != 0 {
                time::sleep(self.shutdown_interval).await;
            }
            logger::info!("Terminating drainer");
            metrics::SUCCESSFUL_SHUTDOWN.add(&metrics::CONTEXT, 1, &[]);
            let shutdown_ended = shutdown_started.elapsed().as_secs_f64() * 1000f64;
            metrics::CLEANUP_TIME.record(&metrics::CONTEXT, shutdown_ended, &[]);
            self.close();
        }
        logger::info!(
            tasks_remaining = self.active_tasks.load(atomic::Ordering::SeqCst),
            "Drainer shutdown successfully"
        )
    }

    pub fn spawn_error_handlers(&self, tx: mpsc::Sender<()>) -> errors::DrainerResult<()> {
        let (redis_error_tx, redis_error_rx) = oneshot::channel();

        let redis_conn_clone = self.stream.store().redis_conn.clone();

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
async fn drainer_handler<T: StreamOperation + Clone + Send + Sync + DrainStream>(
    stream: T,
    stream_index: u8,
    max_read_count: u64,
    active_tasks: Arc<atomic::AtomicU64>,
    jobs_picked: Arc<atomic::AtomicU8>,
) -> errors::DrainerResult<()> {
    active_tasks.fetch_add(1, atomic::Ordering::Release);

    let stream_name = stream.get_drainer_stream_name(stream_index);

    let drainer_result = Box::pin(drainer(
        stream.clone(),
        max_read_count,
        stream_name.as_str(),
        jobs_picked,
    ))
    .await;

    if let Err(error) = drainer_result {
        logger::error!(?error)
    }

    let flag_stream_name = stream.get_stream_key_flag(stream_index);

    let output = stream
        .make_stream_available(flag_stream_name.as_str())
        .await;
    active_tasks.fetch_sub(1, atomic::Ordering::Release);
    output.map_err(|err| {
        logger::error!(operation = "unlock_stream", err=?err);
        err
    })
}

#[instrument(skip_all, fields(global_id, request_id, session_id))]
async fn drainer<T: StreamOperation + Clone + DrainStream + Sync>(
    stream: T,
    max_read_count: u64,
    stream_name: &str,
    jobs_picked: Arc<atomic::AtomicU8>,
) -> errors::DrainerResult<()> {
    let stream_read = match stream.read_from_stream(stream_name, max_read_count).await {
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

    T::drain_stream(entries, stream, stream_name).await?;
    Ok(())
}
