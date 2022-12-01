use actix_web::{web, Scope};

use crate::connection;

use super::health::{default, health, show_payment};
use diesel::{PgConnection, r2d2::{ConnectionManager, Pool}};

#[derive(Clone)]
pub struct Config {
	pub flow_name: String,
  pub conn: Pool<ConnectionManager<PgConnection>>,
}

pub struct Default;

impl Default {
	pub fn server() -> Scope {
		let config = Config {
			flow_name: String::from("default"),
      conn: connection(),
		};

		web::scope("/app")
			.app_data(web::Data::new(config))
			.service(default)
			.service(health)
      .service(show_payment)
	}
	
}
