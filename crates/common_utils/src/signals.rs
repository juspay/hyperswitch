//! Provide Interface for worker services to handle signals

use futures::StreamExt;
use router_env::logger;
pub use tokio::sync::oneshot;

///
pub async fn signal_handler(mut sig: signal_hook_tokio::Signals, sender: oneshot::Sender<()>) {
    if let Some(signal) = sig.next().await {
        logger::info!(
            "Received signal: {:?}",
            signal_hook::low_level::signal_name(signal)
        );
        match signal {
            signal_hook::consts::SIGTERM | signal_hook::consts::SIGINT => match sender.send(()) {
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
pub fn get_allowed_signals() -> Result<signal_hook_tokio::SignalsInfo, std::io::Error> {
    signal_hook_tokio::Signals::new([signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT])
}
