extern crate diesel;

mod types;
mod routes;
mod configs;
mod core;

use actix_web::{
    HttpServer,
    App,
    middleware::Logger
};
use diesel::{
    r2d2::*,
    pg::PgConnection
};

use crate::{
    routes::app::Default,
    configs::settings::Settings
};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let conf = Settings::new().unwrap();
    println!("{:#?}",conf);
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("debug"));
    HttpServer::new(move || {
        App::new()
            .wrap(Logger::new("%a %{User-Agent}i"))
            .service(Default::server())
    })
    .bind((conf.server.host.clone(),conf.server.port))?
    .run()
    .await
}

pub fn connection() -> Pool<ConnectionManager<PgConnection>> {
    let conf = Settings::new().unwrap();
    let database_url = &format!("postgres://{}:{}@{}:{}/{}",
        conf.database.username.clone(),
        conf.database.password.clone(),
        conf.database.host.clone(),
        conf.database.port,
        conf.database.dbname.clone());
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    Pool::builder()
        .max_size(5)
        .build(manager)
        .expect("error in pool")
}
