//! Provide Interface for worker services to handle signals

#[cfg(not(target_os = "windows"))]
use futures::StreamExt;
#[cfg(not(target_os = "windows"))]
use router_env::logger;
use tokio::sync::mpsc;

/// This functions is meant to run in parallel to the application.
/// It will send a signal to the receiver when a SIGTERM or SIGINT is received
#[cfg(not(target_os = "windows"))]
pub async fn signal_handler(mut sig: signal_hook_tokio::Signals, sender: mpsc::Sender<()>) {
    if let Some(signal) = sig.next().await {
        logger::info!(
            "Received signal: {:?}",
            signal_hook::low_level::signal_name(signal)
        );
        match signal {
            signal_hook::consts::SIGTERM | signal_hook::consts::SIGINT => match sender.try_send(())
            {
                Ok(_) => {
                    logger::info!("Request for force shutdown received")
                }
                Err(_) => {
                    logger::error!(
                        "The receiver is closed, a termination call might already be sent"
                    )
                }
            },
            _ => {}
        }
    }
}

/// This functions is meant to run in parallel to the application.
/// It will send a signal to the receiver when a SIGTERM or SIGINT is received
#[cfg(target_os = "windows")]
pub async fn signal_handler(_sig: DummySignal, _sender: mpsc::Sender<()>) {}

/// This function is used to generate a list of signals that the signal_handler should listen for
#[cfg(not(target_os = "windows"))]
pub fn get_allowed_signals() -> Result<signal_hook_tokio::SignalsInfo, std::io::Error> {
    signal_hook_tokio::Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])
}

/// This function is used to generate a list of signals that the signal_handler should listen for
#[cfg(target_os = "windows")]
pub fn get_allowed_signals() -> Result<DummySignal, std::io::Error> {
    Ok(DummySignal)
}

/// Dummy Signal Handler for windows
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct DummySignal;

#[cfg(target_os = "windows")]
impl DummySignal {
    /// Dummy handler for signals in windows (empty)
    pub fn handle(&self) -> Self {
        self.clone()
    }

    /// Hollow implementation, for windows compatibility
    pub fn close(self) {}
}
