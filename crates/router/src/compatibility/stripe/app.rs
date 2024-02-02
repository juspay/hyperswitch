use actix_web::{web, Scope};

use super::{customers::*, payment_intents::*, refunds::*, setup_intents::*, webhooks::*};
use crate::routes::{self, mandates, webhooks};

pub struct PaymentIntents;

impl PaymentIntents {
        /// Defines the server method which creates and configures the routes for payment intents. It takes the app state as input and returns a Scope.
    pub fn server(state: routes::AppState) -> Scope {
        let mut route = web::scope("/payment_intents").app_data(web::Data::new(state));
        #[cfg(feature = "olap")]
        {
            route = route.service(web::resource("/list").route(web::get().to(payment_intent_list)))
        }
        route = route
            .service(web::resource("").route(web::post().to(payment_intents_create)))
            .service(
                web::resource("/sync")
                    .route(web::post().to(payment_intents_retrieve_with_gateway_creds)),
            )
            .service(
                web::resource("/{payment_id}")
                    .route(web::get().to(payment_intents_retrieve))
                    .route(web::post().to(payment_intents_update)),
            )
            .service(
                web::resource("/{payment_id}/confirm")
                    .route(web::post().to(payment_intents_confirm)),
            )
            .service(
                web::resource("/{payment_id}/capture")
                    .route(web::post().to(payment_intents_capture)),
            )
            .service(
                web::resource("/{payment_id}/cancel").route(web::post().to(payment_intents_cancel)),
            );
        route
    }
}

pub struct SetupIntents;

impl SetupIntents {
        /// This method creates a scope for handling setup intents related HTTP requests. It takes in a state of type routes::AppState and returns a Scope. The scope is set up to handle various HTTP methods and routes for setup intents, such as creating, retrieving, updating, and confirming setup intents.
    pub fn server(state: routes::AppState) -> Scope {
        web::scope("/setup_intents")
            .app_data(web::Data::new(state))
            .service(web::resource("").route(web::post().to(setup_intents_create)))
            .service(
                web::resource("/{setup_id}")
                    .route(web::get().to(setup_intents_retrieve))
                    .route(web::post().to(setup_intents_update)),
            )
            .service(
                web::resource("/{setup_id}/confirm").route(web::post().to(setup_intents_confirm)),
            )
    }
}

pub struct Refunds;

impl Refunds {
        /// Defines a server method that creates a scope for handling refund-related requests
    ///
    /// # Arguments
    ///
    /// * `config` - A routes::AppState object containing the configuration for the server
    ///
    /// # Returns
    ///
    /// A Scope object for handling refund-related requests
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/refunds")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(refund_create)))
            .service(
                web::resource("/sync").route(web::post().to(refund_retrieve_with_gateway_creds)),
            )
            .service(
                web::resource("/{refund_id}")
                    .route(web::get().to(refund_retrieve))
                    .route(web::post().to(refund_update)),
            )
    }
}

pub struct Customers;

impl Customers {
        /// Defines the server method that creates a scope for handling customer-related routes.
    /// It takes a routes::AppState configuration as input and sets up routes for creating, retrieving, updating, and deleting customers, as well as listing customer payment methods.
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/customers")
            .app_data(web::Data::new(config))
            .service(web::resource("").route(web::post().to(customer_create)))
            .service(
                web::resource("/{customer_id}")
                    .route(web::get().to(customer_retrieve))
                    .route(web::post().to(customer_update))
                    .route(web::delete().to(customer_delete)),
            )
            .service(
                web::resource("/{customer_id}/payment_methods")
                    .route(web::get().to(list_customer_payment_method_api)),
            )
    }
}

pub struct Webhooks;

impl Webhooks {
        /// This method creates a server for handling incoming webhooks. It takes a `routes::AppState` configuration as input and returns a `Scope` for routing incoming webhooks. The server is scoped under "/webhooks" and includes routes for handling POST and GET requests with merchant_id and connector_name parameters. It uses the `webhooks::receive_incoming_webhook` function to handle incoming webhook requests for the `StripeOutgoingWebhook` type.
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/webhooks")
            .app_data(web::Data::new(config))
            .service(
                web::resource("/{merchant_id}/{connector_name}")
                    .route(
                        web::post().to(webhooks::receive_incoming_webhook::<StripeOutgoingWebhook>),
                    )
                    .route(
                        web::get().to(webhooks::receive_incoming_webhook::<StripeOutgoingWebhook>),
                    ),
            )
    }
}

pub struct Mandates;

impl Mandates {
        /// Create a server with the specified configuration for handling payment methods. 
    pub fn server(config: routes::AppState) -> Scope {
        web::scope("/payment_methods")
            .app_data(web::Data::new(config))
            .service(web::resource("/{id}/detach").route(web::post().to(mandates::revoke_mandate)))
    }
}
