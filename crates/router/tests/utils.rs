#![allow(dead_code)]
use router::{configs::settings::Settings, start_server};
use tokio::sync::OnceCell;

static SERVER: OnceCell<bool> = OnceCell::const_new();

async fn spawn_server() -> bool {
    let conf = Settings::new().expect("invalid settings");
    let (server, _state) = start_server(conf).await.expect("failed to create server");

    let _server = tokio::spawn(server);
    true
}

pub async fn setup() {
    SERVER.get_or_init(spawn_server).await;
}
