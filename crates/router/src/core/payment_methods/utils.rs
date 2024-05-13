use api_models::{
    admin::{self, PaymentMethodsEnabled},
    payment_methods::ResponsePaymentMethodIntermediate,
};
pub use common_enums::enums;
use common_enums::{PaymentMethodType, CaptureMethod};
use common_utils::{
    consts,
    ext_traits::AsyncExt,
};
use error_stack::ResultExt;
use euclid::{frontend::dir::{self, DirValue}, dirval, dssa::graph::*};
use hyperswitch_constraint_graph as cgraph;
use kgraph_utils::error::KgraphError;

use crate::{
    configs::settings,
    core::{
        errors,
        payments::helpers,
    },
    db, routes,
    types,
    utils::OptionExt, services,
};

pub async fn list_payment_methods_from_graph(
    state: routes::AppState,
    merchant_account: types::domain::MerchantAccount,
    key_store: types::domain::MerchantKeyStore,
    req: types::api::PaymentMethodListRequest,
) -> errors::RouterResponse<()>{
    // Db call for all MCAs linked with the merchant account
    let db = &*state.store;
    let pm_config_mapping = &state.conf.pm_filters;
    let _response: Vec<ResponsePaymentMethodIntermediate> = vec![];

    // deriving payment intent from the provided client secret
    let payment_intent = if let Some(cs) = &req.client_secret {
        if cs.starts_with("pm_") {
            validate_payment_method_and_client_secret(cs, db, &merchant_account).await?;
            None
        } else {
            helpers::verify_payment_intent_time_and_client_secret(
                db,
                &merchant_account,
                req.client_secret.clone(),
            )
            .await?
        }
    } else {
        None
    };

    //deriving business country from payment intent
    let _billing_address = payment_intent
        .as_ref()
        .async_map(|pi| async {
            helpers::get_address_by_id(
                db,
                pi.billing_address_id.clone(),
                &key_store,
                &pi.payment_id,
                &merchant_account.merchant_id,
                merchant_account.storage_scheme,
            )
            .await
        })
        .await
        .transpose()?
        .flatten();

    // deriving payment attempt from the provided client secret
    let _payment_attempt = payment_intent
        .as_ref()
        .async_map(|pi| async {
            db.find_payment_attempt_by_payment_id_merchant_id_attempt_id(
                &pi.payment_id,
                &pi.merchant_id,
                &pi.active_attempt.get_id(),
                merchant_account.storage_scheme,
            )
            .await
            .change_context(errors::ApiErrorResponse::PaymentNotFound)
        })
        .await
        .transpose()?;

    let all_mcas = db
        .find_merchant_connector_account_by_merchant_id_and_disabled_list(
            &merchant_account.merchant_id,
            true,
            &key_store,
        )
        .await
        .change_context(errors::ApiErrorResponse::MerchantConnectorAccountNotFound {
            id: merchant_account.merchant_id.clone(),
        })?;

    // separate pamentMethods from those mca
    for mca in &all_mcas {
        let payment_methods = match &mca.payment_methods_enabled {
            Some(pm) => pm.clone(),
            None => continue,
        };
        let memo = &mut cgraph::Memoization::new();
        let cycle_map = &mut cgraph::CycleCheck::new();
        let context = euclid::dssa::graph::AnalysisContext::from_dir_values([
            dirval!(BillingCountry = Australia),
            dirval!(PaymentCurrency = INR),
        ]
        );
        let graph = make_pm_graph(payment_methods, mca.connector_name.clone(), pm_config_mapping)
            .change_context(errors::ApiErrorResponse::InternalServerError)
        .attach_printable("Kgraph isn't working")?;
        let result = graph.key_value_analysis(
            dirval!(CaptureMethod = Automatic),
            &context,
            memo,
            cycle_map,
            None,
        );
        println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>>> result {:?}", result);
    }
Ok(services::ApplicationResponse::StatusOk)
}

fn make_pm_graph<'a> (
    payment_methods:Vec<serde_json::value::Value>,
    connector: String,
    pm_config_mapping: &settings::ConnectorFilters
) -> Result<cgraph::ConstraintGraph<'a, DirValue>, KgraphError> {
    let mut builder = cgraph::ConstraintGraphBuilder::new();
    for payment_method in payment_methods.into_iter() {
        let res = serde_json::from_value::<PaymentMethodsEnabled>(payment_method);
        if let Ok(payment_methods_enabled) = res {
            compile_pm_graph(
                &mut builder,
                payment_methods_enabled.clone(),
                connector.clone(),
                pm_config_mapping,
            )?;
        };
    };
    Ok(builder.build())
}
fn compile_pm_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, DirValue>,
    pm_enabled: PaymentMethodsEnabled,
    connector: String,
    config: &settings::ConnectorFilters,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    // let mut all_node: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    //making countries context
    if let Some(pmt) = pm_enabled.payment_method_types {
        for pmt in pmt {
            if let Some(pm_object_countries) = pmt.accepted_countries {
                agg_nodes.push((
                    compile_accepted_countries_for_mca(
                        builder,
                        &pmt.payment_method_type,
                        pm_object_countries,
                        config,
                        connector.clone(),
                    )?
                    .unwrap(),
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                ))
            }

            // if let Some(pm_object_currencies) = pmt.accepted_currencies {
            //     agg_nodes.push((
            //         compile_accepted_currency_for_mca(
            //             builder,
            //             &pmt.payment_method_type,
            //             pm_object_currencies,
            //             config,
            //             connector.clone(),
            //         )?
            //         .unwrap(),
            //         cgraph::Relation::Positive,
            //         cgraph::Strength::Strong,
            //     ))
            // }
            //
            // all_node.push((builder
            //     .make_all_aggregator(
            //         &agg_nodes,
            //         None,
            //         None::<()>,
            //         None,
            //     )
            //     .map_err(KgraphError::GraphConstructionError)?,
            //     cgraph::Relation::Positive,
            //     cgraph::Strength::Strong,
            // ));
        }
    }

    let any_aggregator = builder
        .make_any_aggregator(&agg_nodes, None, None::<()>, None)
        .expect("failed to make all aggregator");

    // Making our output node
    let pm_info = "PaymentMethod";
    let pm_node = builder.make_value_node(
        // cgraph::NodeValue::Value(DirValue::PaymentMethod(pm_enabled.payment_method)),
        cgraph::NodeValue::Value(DirValue::CaptureMethod(CaptureMethod::Automatic)),
        Some(pm_info),
        None::<()>,
    );

    builder.make_edge(
        any_aggregator,
        pm_node,
        cgraph::Strength::Strong,
        cgraph::Relation::Positive,
        None::<cgraph::DomainId>,
    ).expect("failed to make edge");

    Ok(None)
}

// fn compile_accepted_currency_for_mca(
//     builder: &mut cgraph::ConstraintGraphBuilder<'_, DirValue>,
//     payment_method_type: &PaymentMethodType,
//     pm_obj_currency: admin::AcceptedCurrencies,
//     config: &settings::ConnectorFilters,
//     connector: String,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     match pm_obj_currency {
//         admin::AcceptedCurrencies::EnableOnly(currency) => {
//             if let Some(config) = config
//                 .0
//                 .get(connector.as_str())
//                 .or_else(|| config.0.get("default"))
//             {
//                 if let Some(value) =
//                     config
//                         .0
//                         .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
//                             *payment_method_type,
//                         ))
//                 {
//                     println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>mca countries {:?}", currency);
//                     // Currency from the MCA
//                     let pm_object_currency_value_node = builder
//                         .make_in_aggregator(
//                             currency
//                                 .into_iter()
//                                 .map(|currency| {
//                                     dir::DirValue::PaymentCurrency (currency)
//                                 })
//                                 .collect(),
//                             None,
//                             None::<()>,
//                         )
//                         .expect("error1");
//
//                     // Currency from config
//                     let config_currency: Vec<common_enums::Currency> = 
//                         Vec::from_iter(value.currency.clone().unwrap())
//                             .into_iter()
//                             .collect();
//
//                     println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>config countries {:?}", config_currency);
//                     let dir_currencies: Vec<DirValue> = config_currency.into_iter()
//                         .map(|currency| dir::DirValue::PaymentCurrency(currency))
//                         .collect();
//
//                     let config_country_agg_node = builder
//                         .make_in_aggregator(dir_currencies, None, None::<()>)
//                         .unwrap();
//
//                     let node = builder
//                         .make_all_aggregator(
//                             &[
//                                 (
//                                     pm_object_currency_value_node,
//                                     cgraph::Relation::Positive,
//                                     cgraph::Strength::Strong,
//                                 ),
//                                 (
//                                     config_country_agg_node,
//                                     cgraph::Relation::Positive,
//                                     cgraph::Strength::Strong,
//                                 ),
//                             ],
//                             None,
//                             None::<()>,
//                             None,
//                         )
//                         .map_err(KgraphError::GraphConstructionError)?;
//                     return Ok(Some(node));
//                 }
//             }
//         }
//         admin::AcceptedCurrencies::DisableOnly(_) => todo!(),
//         admin::AcceptedCurrencies::AllAccepted => todo!(),
//     }
//     Ok(None)
// }

fn compile_accepted_countries_for_mca(
    builder: &mut cgraph::ConstraintGraphBuilder<'_, DirValue>,
    payment_method_type: &PaymentMethodType,
    pm_obj_countries: admin::AcceptedCountries,
    config: &settings::ConnectorFilters,
    connector: String,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    match pm_obj_countries {
        admin::AcceptedCountries::EnableOnly(countries) => {
            if let Some(config) = config
                .0
                .get(connector.as_str())
                .or_else(|| config.0.get("default"))
            {
                if let Some(value) =
                    config
                        .0
                        .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
                            *payment_method_type,
                        ))
                {
                    println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>mca countries {:?}", countries);
                    // Country from the MCA
                    let pm_object_country_value_node = builder
                        .make_in_aggregator(
                            countries
                                .into_iter()
                                .map(|country| {
                                    dir::DirValue::BillingCountry(
                                        common_enums::Country::from_alpha2(country),
                                    )
                                })
                                .collect(),
                            None,
                            None::<()>,
                        )
                        .expect("error1");

                    // country from config
                    let config_countries: Vec<common_enums::Country> =
                        Vec::from_iter(value.country.as_ref().unwrap())
                            .into_iter()
                            .map(|country| common_enums::Country::from_alpha2(*country))
                            .collect();
                    println!(">>>>>>>>>>>>>>>>>>>>>>>>>>>config countries {:?}", config_countries);
                    let dir_countries: Vec<DirValue> = config_countries
                        .into_iter()
                        .map(|country| dir::DirValue::BillingCountry(country))
                        .collect();
                    let config_country_agg_node = builder
                        .make_in_aggregator(dir_countries, None, None::<()>)
                        .unwrap();
                    let node = builder
                        .make_all_aggregator(
                            &[
                                (
                                    pm_object_country_value_node,
                                    cgraph::Relation::Positive,
                                    cgraph::Strength::Strong,
                                ),
                                (
                                    config_country_agg_node,
                                    cgraph::Relation::Positive,
                                    cgraph::Strength::Strong,
                                ),
                            ],
                            None,
                            None::<()>,
                            None,
                        )
                        .map_err(KgraphError::GraphConstructionError)?;
                    return Ok(Some(node));
                }
            }
        }
        admin::AcceptedCountries::DisableOnly(_) => todo!(),
        admin::AcceptedCountries::AllAccepted => todo!(),
    }
    Ok(None)
}

//****************************************************Helper functions*******************************
async fn validate_payment_method_and_client_secret(
    cs: &String,
    db: &dyn db::StorageInterface,
    merchant_account: &types::domain::MerchantAccount,
) -> Result<(), error_stack::Report<errors::ApiErrorResponse>> {
    let pm_vec = cs.split("_secret").collect::<Vec<&str>>();
    let pm_id = pm_vec
        .first()
        .ok_or(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })?;

    let payment_method = db
        .find_payment_method(pm_id, merchant_account.storage_scheme)
        .await
        .change_context(errors::ApiErrorResponse::PaymentMethodNotFound)
        .attach_printable("Unable to find payment method")?;

    let client_secret_expired =
        authenticate_pm_client_secret_and_check_expiry(cs, &payment_method)?;
    if client_secret_expired {
        return Err::<(), error_stack::Report<errors::ApiErrorResponse>>(
            (errors::ApiErrorResponse::ClientSecretExpired).into(),
        );
    }
    Ok(())
}

pub fn authenticate_pm_client_secret_and_check_expiry(
    req_client_secret: &String,
    payment_method: &diesel_models::PaymentMethod,
) -> errors::CustomResult<bool, errors::ApiErrorResponse> {
    let stored_client_secret = payment_method
        .client_secret
        .clone()
        .get_required_value("client_secret")
        .change_context(errors::ApiErrorResponse::MissingRequiredField {
            field_name: "client_secret",
        })
        .attach_printable("client secret not found in db")?;

    if req_client_secret != &stored_client_secret {
        Err((errors::ApiErrorResponse::ClientSecretInvalid).into())
    } else {
        let current_timestamp = common_utils::date_time::now();
        let session_expiry = payment_method
            .created_at
            .saturating_add(time::Duration::seconds(consts::DEFAULT_SESSION_EXPIRY));

        let expired = current_timestamp > session_expiry;

        Ok(expired)
    }
}
