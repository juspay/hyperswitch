//! Provide Interface for worker services to handle signals

#[cfg(not(target_os = "windows"))]
use futures::StreamExt;
#[cfg(not(target_os = "windows"))]
use router_env::logger;
use tokio::sync::mpsc;

///
/// This functions is meant to run in parallel to the application.
/// It will send a signal to the receiver when a SIGTERM or SIGINT is received
///
#[cfg(not(target_os = "windows"))]
/// Listens for signals using signal_hook_tokio and sends a corresponding message through the provided sender channel when a signal is received. 
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

///
/// This functions is meant to run in parallel to the application.
/// It will send a signal to the receiver when a SIGTERM or SIGINT is received
///
#[cfg(target_os = "windows")]
/// Handles the signal event by sending a message to the provided sender.
pub async fn signal_handler(_sig: DummySignal, _sender: mpsc::Sender<()>) {}

///
/// This function is used to generate a list of signals that the signal_handler should listen for
///
#[cfg(not(target_os = "windows"))]
/// Retrieves the allowed signals for the application to handle using signal_hook_tokio.
///
/// # Returns
///
/// A Result containing the SignalsInfo struct that provides information about the allowed signals, or an Error if there was an issue retrieving the signals.
pub fn get_allowed_signals() -> Result<signal_hook_tokio::SignalsInfo, std::io::Error> {
    signal_hook_tokio::Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])
}

///
/// This function is used to generate a list of signals that the signal_handler should listen for
///
#[cfg(target_os = "windows")]
/// Retrieves the allowed signals for the DummySignal.
/// 
/// # Returns
/// 
/// - `Result<DummySignal, std::io::Error>`: A Result containing the allowed DummySignal if successful, or an std::io::Error if an error occurred.
pub fn get_allowed_signals() -> Result<DummySignal, std::io::Error> {
    Ok(DummySignal)
}

///
/// Dummy Signal Handler for windows
///
#[cfg(target_os = "windows")]
#[derive(Debug, Clone)]
pub struct DummySignal;

#[cfg(target_os = "windows")]
impl DummySignal {
    ///
    /// Dummy handler for signals in windows (empty)
    ///
    pub fn handle(&self) -> Self {
        self.clone()
    }

    ///
    /// Hollow implementation, for windows compatibility
    ///
    pub fn close(self) {}
}
