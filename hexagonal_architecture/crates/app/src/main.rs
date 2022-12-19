use actix_web::HttpServer;
use hexagonal_architecture::config::{dotenv_with_proof, Config};
use hexagonal_architecture::mk_app;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let proof = dotenv_with_proof().unwrap();
    let config = Config::new(&proof).unwrap();

    HttpServer::new(mk_app).bind(config.application_url)?.run().await
}
