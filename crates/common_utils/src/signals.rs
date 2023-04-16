//! Provide Interface for worker services to handle signals

use futures::StreamExt;
use router_env::logger;
use tokio::sync::mpsc;

///
/// This functions is meant to run in parallel to the application.
/// It will send a signal to the receiver when a SIGTERM or SIGINT is received
///
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
/// This function is used to generate a list of signals that the signal_handler should listen for
///
pub fn get_allowed_signals() -> Result<signal_hook_tokio::SignalsInfo, std::io::Error> {
    signal_hook_tokio::Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])
}
