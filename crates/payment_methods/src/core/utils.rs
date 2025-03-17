use std::{
    collections::{HashMap, HashSet},
    str::FromStr,
};

pub use common_utils::{
    crypto::{self, Encryptable},
    ext_traits::{ByteSliceExt, BytesExt, Encode, StringExt, ValueExt},
    fp_utils::when,
    id_type, pii,
    validation::validate_email,
};
use masking::Deserialize;

// pub fn make_pm_graph(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     domain_id: cgraph::DomainId,
//     payment_methods: &[masking::Secret<serde_json::value::Value>],
//     connector: String,
//     pm_config_mapping: &settings::ConnectorFilters,
//     supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
//     supported_payment_methods_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
// ) -> Result<(), KgraphError> {
//     for payment_method in payment_methods.iter() {
//         let pm_enabled =
//             serde_json::from_value::<PaymentMethodsEnabled>(payment_method.clone().expose());
//         if let Ok(payment_methods_enabled) = pm_enabled {
//             compile_pm_graph(
//                 builder,
//                 domain_id,
//                 payment_methods_enabled.clone(),
//                 connector.clone(),
//                 pm_config_mapping,
//                 supported_payment_methods_for_mandate,
//                 supported_payment_methods_for_update_mandate,
//             )?;
//         };
//     }
//     Ok(())
// }

// pub async fn get_merchant_pm_filter_graph(
//     state: &PaymentMethodsState,
//     key: &str,
// ) -> Option<Arc<hyperswitch_constraint_graph::ConstraintGraph<dir::DirValue>>> {
//     PM_FILTERS_CGRAPH_CACHE
//         .get_val::<Arc<hyperswitch_constraint_graph::ConstraintGraph<dir::DirValue>>>(CacheKey {
//             key: key.to_string(),
//             prefix: state.tenant.redis_key_prefix.clone(),
//         })
//         .await
// }

// pub async fn refresh_pm_filters_cache(
//     state: &PaymentMethodsState,
//     key: &str,
//     graph: cgraph::ConstraintGraph<dir::DirValue>,
// ) -> Arc<hyperswitch_constraint_graph::ConstraintGraph<dir::DirValue>> {
//     let pm_filter_graph = Arc::new(graph);
//     PM_FILTERS_CGRAPH_CACHE
//         .push(
//             CacheKey {
//                 key: key.to_string(),
//                 prefix: state.tenant.redis_key_prefix.clone(),
//             },
//             pm_filter_graph.clone(),
//         )
//         .await;
//     pm_filter_graph
// }

// fn compile_pm_graph(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     domain_id: cgraph::DomainId,
//     pm_enabled: PaymentMethodsEnabled,
//     connector: String,
//     config: &settings::ConnectorFilters,
//     supported_payment_methods_for_mandate: &settings::SupportedPaymentMethodsForMandate,
//     supported_payment_methods_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
// ) -> Result<(), KgraphError> {
//     if let Some(payment_method_types) = pm_enabled.payment_method_types {
//         for pmt in payment_method_types {
//             let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> =
//                 Vec::new();
//             let mut agg_or_nodes_for_mandate_filters: Vec<(
//                 cgraph::NodeId,
//                 cgraph::Relation,
//                 cgraph::Strength,
//             )> = Vec::new();

//             // Connector supported for Update mandate filter
//             let res = construct_supported_connectors_for_update_mandate_node(
//                 builder,
//                 domain_id,
//                 supported_payment_methods_for_update_mandate,
//                 pmt.clone(),
//                 pm_enabled.payment_method,
//             );
//             if let Ok(Some(connector_eligible_for_update_mandates_node)) = res {
//                 agg_or_nodes_for_mandate_filters.push((
//                     connector_eligible_for_update_mandates_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Strong,
//                 ))
//             }

//             // Connector supported for mandates filter
//             if let Some(supported_pm_for_mandates) = supported_payment_methods_for_mandate
//                 .0
//                 .get(&pm_enabled.payment_method)
//             {
//                 if let Some(supported_connector_for_mandates) =
//                     supported_pm_for_mandates.0.get(&pmt.payment_method_type)
//                 {
//                     let supported_connectors: Vec<api_enums::Connector> =
//                         supported_connector_for_mandates
//                             .connector_list
//                             .clone()
//                             .into_iter()
//                             .collect();
//                     if let Ok(Some(connector_eligible_for_mandates_node)) =
//                         construct_supported_connectors_for_mandate_node(
//                             builder,
//                             domain_id,
//                             supported_connectors,
//                         )
//                     {
//                         agg_or_nodes_for_mandate_filters.push((
//                             connector_eligible_for_mandates_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ))
//                     }
//                 }
//             }

//             // Non Prominent Mandate flows
//             let payment_type_non_mandate_value_node = builder.make_value_node(
//                 cgraph::NodeValue::Value(dir::DirValue::PaymentType(
//                     euclid::enums::PaymentType::NonMandate,
//                 )),
//                 None,
//                 None::<()>,
//             );
//             let payment_type_setup_mandate_value_node = builder.make_value_node(
//                 cgraph::NodeValue::Value(dir::DirValue::PaymentType(
//                     euclid::enums::PaymentType::SetupMandate,
//                 )),
//                 None,
//                 None::<()>,
//             );

//             let non_major_mandate_any_node = builder
//                 .make_any_aggregator(
//                     &[
//                         (
//                             payment_type_non_mandate_value_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                         (
//                             payment_type_setup_mandate_value_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                     ],
//                     None,
//                     None::<()>,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?;

//             agg_or_nodes_for_mandate_filters.push((
//                 non_major_mandate_any_node,
//                 cgraph::Relation::Positive,
//                 cgraph::Strength::Strong,
//             ));

//             let agg_or_node = builder
//                 .make_any_aggregator(
//                     &agg_or_nodes_for_mandate_filters,
//                     None,
//                     None::<()>,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?;

//             agg_nodes.push((
//                 agg_or_node,
//                 cgraph::Relation::Positive,
//                 cgraph::Strength::Strong,
//             ));

//             // Capture Method filter
//             config
//                 .0
//                 .get(connector.as_str())
//                 .or_else(|| config.0.get("default"))
//                 .map(|inner| {
//                     if let Ok(Some(capture_method_filter)) =
//                         construct_capture_method_node(builder, inner, pmt.payment_method_type)
//                     {
//                         agg_nodes.push((
//                             capture_method_filter,
//                             cgraph::Relation::Negative,
//                             cgraph::Strength::Strong,
//                         ))
//                     }
//                 });

//             // Country filter
//             if let Ok(Some(country_node)) = compile_accepted_countries_for_mca(
//                 builder,
//                 domain_id,
//                 pmt.payment_method_type,
//                 pmt.accepted_countries,
//                 config,
//                 connector.clone(),
//             ) {
//                 agg_nodes.push((
//                     country_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Strong,
//                 ))
//             }

//             // Currency filter
//             if let Ok(Some(currency_node)) = compile_accepted_currency_for_mca(
//                 builder,
//                 domain_id,
//                 pmt.payment_method_type,
//                 pmt.accepted_currencies,
//                 config,
//                 connector.clone(),
//             ) {
//                 agg_nodes.push((
//                     currency_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Strong,
//                 ))
//             }

//             let and_node_for_all_the_filters = builder
//                 .make_all_aggregator(&agg_nodes, None, None::<()>, Some(domain_id))
//                 .map_err(KgraphError::GraphConstructionError)?;

//             // Making our output node
//             let pmt_info = "PaymentMethodType";
//             let dir_node: cgraph::NodeValue<dir::DirValue> =
//                 (pmt.payment_method_type, pm_enabled.payment_method)
//                     .into_dir_value()
//                     .map(Into::into)?;
//             let payment_method_type_value_node =
//                 builder.make_value_node(dir_node, Some(pmt_info), None::<()>);

//             builder
//                 .make_edge(
//                     and_node_for_all_the_filters,
//                     payment_method_type_value_node,
//                     cgraph::Strength::Normal,
//                     cgraph::Relation::Positive,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?;
//         }
//     }
//     Ok(())
// }

// fn construct_supported_connectors_for_update_mandate_node(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     domain_id: cgraph::DomainId,
//     supported_payment_methods_for_update_mandate: &settings::SupportedPaymentMethodsForMandate,
//     pmt: RequestPaymentMethodTypes,
//     payment_method: enums::PaymentMethod,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     let card_value_node = builder.make_value_node(
//         cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(enums::PaymentMethod::Card)),
//         None,
//         None::<()>,
//     );

//     let payment_type_value_node = builder.make_value_node(
//         cgraph::NodeValue::Value(dir::DirValue::PaymentType(
//             euclid::enums::PaymentType::UpdateMandate,
//         )),
//         None,
//         None::<()>,
//     );

//     let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
//     let mut card_dir_values = Vec::new();
//     let mut non_card_dir_values = Vec::new();

//     if let Some(supported_pm_for_mandates) = supported_payment_methods_for_update_mandate
//         .0
//         .get(&payment_method)
//     {
//         if payment_method == enums::PaymentMethod::Card {
//             if let Some(credit_connector_list) = supported_pm_for_mandates
//                 .0
//                 .get(&api_enums::PaymentMethodType::Credit)
//             {
//                 card_dir_values.extend(
//                     credit_connector_list
//                         .connector_list
//                         .clone()
//                         .into_iter()
//                         .filter_map(|connector| {
//                             api_enums::RoutableConnectors::from_str(connector.to_string().as_str())
//                                 .ok()
//                                 .map(|connector| {
//                                     dir::DirValue::Connector(Box::new(
//                                         api_models::routing::ast::ConnectorChoice { connector },
//                                     ))
//                                 })
//                         }),
//                 );
//             }

//             if let Some(debit_connector_list) = supported_pm_for_mandates
//                 .0
//                 .get(&api_enums::PaymentMethodType::Debit)
//             {
//                 card_dir_values.extend(
//                     debit_connector_list
//                         .connector_list
//                         .clone()
//                         .into_iter()
//                         .filter_map(|connector| {
//                             api_enums::RoutableConnectors::from_str(connector.to_string().as_str())
//                                 .ok()
//                                 .map(|connector| {
//                                     dir::DirValue::Connector(Box::new(
//                                         api_models::routing::ast::ConnectorChoice { connector },
//                                     ))
//                                 })
//                         }),
//                 );
//             }
//             let card_in_node = builder
//                 .make_in_aggregator(card_dir_values, None, None::<()>)
//                 .map_err(KgraphError::GraphConstructionError)?;

//             let card_and_node = builder
//                 .make_all_aggregator(
//                     &[
//                         (
//                             card_value_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                         (
//                             payment_type_value_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                         (
//                             card_in_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                     ],
//                     None,
//                     None::<()>,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?;

//             agg_nodes.push((
//                 card_and_node,
//                 cgraph::Relation::Positive,
//                 cgraph::Strength::Strong,
//             ));
//         } else if let Some(connector_list) =
//             supported_pm_for_mandates.0.get(&pmt.payment_method_type)
//         {
//             non_card_dir_values.extend(
//                 connector_list
//                     .connector_list
//                     .clone()
//                     .into_iter()
//                     .filter_map(|connector| {
//                         api_enums::RoutableConnectors::from_str(connector.to_string().as_str())
//                             .ok()
//                             .map(|connector| {
//                                 dir::DirValue::Connector(Box::new(
//                                     api_models::routing::ast::ConnectorChoice { connector },
//                                 ))
//                             })
//                     }),
//             );
//             let non_card_mandate_in_node = builder
//                 .make_in_aggregator(non_card_dir_values, None, None::<()>)
//                 .map_err(KgraphError::GraphConstructionError)?;

//             let non_card_and_node = builder
//                 .make_all_aggregator(
//                     &[
//                         (
//                             card_value_node,
//                             cgraph::Relation::Negative,
//                             cgraph::Strength::Strong,
//                         ),
//                         (
//                             payment_type_value_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                         (
//                             non_card_mandate_in_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                     ],
//                     None,
//                     None::<()>,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?;

//             agg_nodes.push((
//                 non_card_and_node,
//                 cgraph::Relation::Positive,
//                 cgraph::Strength::Strong,
//             ));
//         }
//     }

//     if !agg_nodes.is_empty() {
//         Ok(Some(
//             builder
//                 .make_any_aggregator(
//                     &agg_nodes,
//                     Some("any node for card and non card pm"),
//                     None::<()>,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?,
//         ))
//     } else {
//         Ok(None)
//     }
// }

// fn construct_supported_connectors_for_mandate_node(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     domain_id: cgraph::DomainId,
//     eligible_connectors: Vec<api_enums::Connector>,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     let payment_type_value_node = builder.make_value_node(
//         cgraph::NodeValue::Value(dir::DirValue::PaymentType(
//             euclid::enums::PaymentType::NewMandate,
//         )),
//         None,
//         None::<()>,
//     );
//     let connectors_from_config: Vec<dir::DirValue> = eligible_connectors
//         .into_iter()
//         .filter_map(|connector| {
//             match api_enums::RoutableConnectors::from_str(connector.to_string().as_str()) {
//                 Ok(connector) => Some(dir::DirValue::Connector(Box::new(
//                     api_models::routing::ast::ConnectorChoice { connector },
//                 ))),
//                 Err(_) => None,
//             }
//         })
//         .collect();

//     if connectors_from_config.is_empty() {
//         Ok(None)
//     } else {
//         let connector_in_aggregator = builder
//             .make_in_aggregator(connectors_from_config, None, None::<()>)
//             .map_err(KgraphError::GraphConstructionError)?;
//         Ok(Some(
//             builder
//                 .make_all_aggregator(
//                     &[
//                         (
//                             payment_type_value_node,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                         (
//                             connector_in_aggregator,
//                             cgraph::Relation::Positive,
//                             cgraph::Strength::Strong,
//                         ),
//                     ],
//                     None,
//                     None::<()>,
//                     Some(domain_id),
//                 )
//                 .map_err(KgraphError::GraphConstructionError)?,
//         ))
//     }
// }

// fn construct_capture_method_node(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     payment_method_filters: &settings::PaymentMethodFilters,
//     payment_method_type: api_enums::PaymentMethodType,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     if !payment_method_filters
//         .0
//         .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
//             payment_method_type,
//         ))
//         .and_then(|v| v.not_available_flows)
//         .and_then(|v| v.capture_method)
//         .map(|v| !matches!(v, api_enums::CaptureMethod::Manual))
//         .unwrap_or(true)
//     {
//         return Ok(Some(builder.make_value_node(
//             cgraph::NodeValue::Value(dir::DirValue::CaptureMethod(
//                 common_enums::CaptureMethod::Manual,
//             )),
//             None,
//             None::<()>,
//         )));
//     }
//     Ok(None)
// }

// // fn construct_card_network_nodes(
// //     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
// //     mca_card_networks: Vec<api_enums::CardNetwork>,
// // ) -> Result<Option<cgraph::NodeId>, KgraphError> {
// //     Ok(Some(
// //         builder
// //             .make_in_aggregator(
// //                 mca_card_networks
// //                     .into_iter()
// //                     .map(dir::DirValue::CardNetwork)
// //                     .collect(),
// //                 None,
// //                 None::<()>,
// //             )
// //             .map_err(KgraphError::GraphConstructionError)?,
// //     ))
// // }

// fn compile_accepted_countries_for_mca(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     domain_id: cgraph::DomainId,
//     payment_method_type: enums::PaymentMethodType,
//     pm_countries: Option<admin::AcceptedCountries>,
//     config: &settings::ConnectorFilters,
//     connector: String,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

//     // Country from the MCA
//     if let Some(pm_obj_countries) = pm_countries {
//         match pm_obj_countries {
//             admin::AcceptedCountries::EnableOnly(countries) => {
//                 let pm_object_country_value_node = builder
//                     .make_in_aggregator(
//                         countries
//                             .into_iter()
//                             .map(|country| {
//                                 dir::DirValue::BillingCountry(common_enums::Country::from_alpha2(
//                                     country,
//                                 ))
//                             })
//                             .collect(),
//                         None,
//                         None::<()>,
//                     )
//                     .map_err(KgraphError::GraphConstructionError)?;
//                 agg_nodes.push((
//                     pm_object_country_value_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Weak,
//                 ));
//             }
//             admin::AcceptedCountries::DisableOnly(countries) => {
//                 let pm_object_country_value_node = builder
//                     .make_in_aggregator(
//                         countries
//                             .into_iter()
//                             .map(|country| {
//                                 dir::DirValue::BillingCountry(common_enums::Country::from_alpha2(
//                                     country,
//                                 ))
//                             })
//                             .collect(),
//                         None,
//                         None::<()>,
//                     )
//                     .map_err(KgraphError::GraphConstructionError)?;
//                 agg_nodes.push((
//                     pm_object_country_value_node,
//                     cgraph::Relation::Negative,
//                     cgraph::Strength::Weak,
//                 ));
//             }
//             admin::AcceptedCountries::AllAccepted => return Ok(None),
//         }
//     }

//     // country from config
//     if let Some(derived_config) = config
//         .0
//         .get(connector.as_str())
//         .or_else(|| config.0.get("default"))
//     {
//         if let Some(value) =
//             derived_config
//                 .0
//                 .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
//                     payment_method_type,
//                 ))
//         {
//             if let Some(config_countries) = value.country.as_ref() {
//                 let config_countries: Vec<common_enums::Country> = Vec::from_iter(config_countries)
//                     .into_iter()
//                     .map(|country| common_enums::Country::from_alpha2(*country))
//                     .collect();
//                 let dir_countries: Vec<dir::DirValue> = config_countries
//                     .into_iter()
//                     .map(dir::DirValue::BillingCountry)
//                     .collect();

//                 let config_country_agg_node = builder
//                     .make_in_aggregator(dir_countries, None, None::<()>)
//                     .map_err(KgraphError::GraphConstructionError)?;

//                 agg_nodes.push((
//                     config_country_agg_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Weak,
//                 ));
//             }
//         } else if let Some(default_derived_config) = config.0.get("default") {
//             if let Some(value) =
//                 default_derived_config
//                     .0
//                     .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
//                         payment_method_type,
//                     ))
//             {
//                 if let Some(config_countries) = value.country.as_ref() {
//                     let config_countries: Vec<common_enums::Country> =
//                         Vec::from_iter(config_countries)
//                             .into_iter()
//                             .map(|country| common_enums::Country::from_alpha2(*country))
//                             .collect();
//                     let dir_countries: Vec<dir::DirValue> = config_countries
//                         .into_iter()
//                         .map(dir::DirValue::BillingCountry)
//                         .collect();

//                     let config_country_agg_node = builder
//                         .make_in_aggregator(dir_countries, None, None::<()>)
//                         .map_err(KgraphError::GraphConstructionError)?;

//                     agg_nodes.push((
//                         config_country_agg_node,
//                         cgraph::Relation::Positive,
//                         cgraph::Strength::Weak,
//                     ));
//                 }
//             }
//         };
//     }
//     Ok(Some(
//         builder
//             .make_all_aggregator(&agg_nodes, None, None::<()>, Some(domain_id))
//             .map_err(KgraphError::GraphConstructionError)?,
//     ))
// }

// fn compile_accepted_currency_for_mca(
//     builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
//     domain_id: cgraph::DomainId,
//     payment_method_type: enums::PaymentMethodType,
//     pm_currency: Option<admin::AcceptedCurrencies>,
//     config: &settings::ConnectorFilters,
//     connector: String,
// ) -> Result<Option<cgraph::NodeId>, KgraphError> {
//     let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
//     // Currency from the MCA
//     if let Some(pm_obj_currency) = pm_currency {
//         match pm_obj_currency {
//             admin::AcceptedCurrencies::EnableOnly(currency) => {
//                 let pm_object_currency_value_node = builder
//                     .make_in_aggregator(
//                         currency
//                             .into_iter()
//                             .map(dir::DirValue::PaymentCurrency)
//                             .collect(),
//                         None,
//                         None::<()>,
//                     )
//                     .map_err(KgraphError::GraphConstructionError)?;
//                 agg_nodes.push((
//                     pm_object_currency_value_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Weak,
//                 ));
//             }
//             admin::AcceptedCurrencies::DisableOnly(currency) => {
//                 let pm_object_currency_value_node = builder
//                     .make_in_aggregator(
//                         currency
//                             .into_iter()
//                             .map(dir::DirValue::PaymentCurrency)
//                             .collect(),
//                         None,
//                         None::<()>,
//                     )
//                     .map_err(KgraphError::GraphConstructionError)?;
//                 agg_nodes.push((
//                     pm_object_currency_value_node,
//                     cgraph::Relation::Negative,
//                     cgraph::Strength::Weak,
//                 ));
//             }
//             admin::AcceptedCurrencies::AllAccepted => return Ok(None),
//         }
//     }

//     // currency from config
//     if let Some(derived_config) = config
//         .0
//         .get(connector.as_str())
//         .or_else(|| config.0.get("default"))
//     {
//         if let Some(value) =
//             derived_config
//                 .0
//                 .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
//                     payment_method_type,
//                 ))
//         {
//             if let Some(config_currencies) = value.currency.as_ref() {
//                 let config_currency: Vec<common_enums::Currency> =
//                     Vec::from_iter(config_currencies)
//                         .into_iter()
//                         .copied()
//                         .collect();

//                 let dir_currencies: Vec<dir::DirValue> = config_currency
//                     .into_iter()
//                     .map(dir::DirValue::PaymentCurrency)
//                     .collect();

//                 let config_currency_agg_node = builder
//                     .make_in_aggregator(dir_currencies, None, None::<()>)
//                     .map_err(KgraphError::GraphConstructionError)?;

//                 agg_nodes.push((
//                     config_currency_agg_node,
//                     cgraph::Relation::Positive,
//                     cgraph::Strength::Weak,
//                 ));
//             }
//         } else if let Some(default_derived_config) = config.0.get("default") {
//             if let Some(value) =
//                 default_derived_config
//                     .0
//                     .get(&settings::PaymentMethodFilterKey::PaymentMethodType(
//                         payment_method_type,
//                     ))
//             {
//                 if let Some(config_currencies) = value.currency.as_ref() {
//                     let config_currency: Vec<common_enums::Currency> =
//                         Vec::from_iter(config_currencies)
//                             .into_iter()
//                             .copied()
//                             .collect();

//                     let dir_currencies: Vec<dir::DirValue> = config_currency
//                         .into_iter()
//                         .map(dir::DirValue::PaymentCurrency)
//                         .collect();

//                     let config_currency_agg_node = builder
//                         .make_in_aggregator(dir_currencies, None, None::<()>)
//                         .map_err(KgraphError::GraphConstructionError)?;

//                     agg_nodes.push((
//                         config_currency_agg_node,
//                         cgraph::Relation::Positive,
//                         cgraph::Strength::Weak,
//                     ))
//                 }
//             }
//         };
//     }
//     Ok(Some(
//         builder
//             .make_all_aggregator(&agg_nodes, None, None::<()>, Some(domain_id))
//             .map_err(KgraphError::GraphConstructionError)?,
//     ))
// }

pub trait ForeignTryFrom<F>: Sized {
    type Error;

    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

fn deserialize_hashmap_inner<K, V>(
    value: HashMap<String, String>,
) -> Result<HashMap<K, HashSet<V>>, String>
where
    K: Eq + FromStr + std::hash::Hash,
    V: Eq + FromStr + std::hash::Hash,
    <K as FromStr>::Err: std::fmt::Display,
    <V as FromStr>::Err: std::fmt::Display,
{
    let (values, errors) = value
        .into_iter()
        .map(
            |(k, v)| match (K::from_str(k.trim()), deserialize_hashset_inner(v)) {
                (Err(error), _) => Err(format!(
                    "Unable to deserialize `{}` as `{}`: {error}",
                    k,
                    std::any::type_name::<K>()
                )),
                (_, Err(error)) => Err(error),
                (Ok(key), Ok(value)) => Ok((key, value)),
            },
        )
        .fold(
            (HashMap::new(), Vec::new()),
            |(mut values, mut errors), result| match result {
                Ok((key, value)) => {
                    values.insert(key, value);
                    (values, errors)
                }
                Err(error) => {
                    errors.push(error);
                    (values, errors)
                }
            },
        );
    if !errors.is_empty() {
        Err(format!("Some errors occurred:\n{}", errors.join("\n")))
    } else {
        Ok(values)
    }
}

pub fn deserialize_hashmap<'a, D, K, V>(deserializer: D) -> Result<HashMap<K, HashSet<V>>, D::Error>
where
    D: serde::Deserializer<'a>,
    K: Eq + FromStr + std::hash::Hash,
    V: Eq + FromStr + std::hash::Hash,
    <K as FromStr>::Err: std::fmt::Display,
    <V as FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;
    deserialize_hashmap_inner(<HashMap<String, String>>::deserialize(deserializer)?)
        .map_err(D::Error::custom)
}

fn deserialize_hashset_inner<T>(value: impl AsRef<str>) -> Result<HashSet<T>, String>
where
    T: Eq + FromStr + std::hash::Hash,
    <T as FromStr>::Err: std::fmt::Display,
{
    let (values, errors) = value
        .as_ref()
        .trim()
        .split(',')
        .map(|s| {
            T::from_str(s.trim()).map_err(|error| {
                format!(
                    "Unable to deserialize `{}` as `{}`: {error}",
                    s.trim(),
                    std::any::type_name::<T>()
                )
            })
        })
        .fold(
            (HashSet::new(), Vec::new()),
            |(mut values, mut errors), result| match result {
                Ok(t) => {
                    values.insert(t);
                    (values, errors)
                }
                Err(error) => {
                    errors.push(error);
                    (values, errors)
                }
            },
        );
    if !errors.is_empty() {
        Err(format!("Some errors occurred:\n{}", errors.join("\n")))
    } else {
        Ok(values)
    }
}

#[cfg(test)]
mod hashmap_deserialization_test {
    #![allow(clippy::unwrap_used)]
    use std::collections::{HashMap, HashSet};

    use serde::de::{
        value::{Error as ValueError, MapDeserializer},
        IntoDeserializer,
    };

    use super::deserialize_hashmap;

    #[test]
    fn test_payment_method_and_payment_method_types() {
        use diesel_models::enums::{PaymentMethod, PaymentMethodType};

        let input_map: HashMap<String, String> = HashMap::from([
            ("bank_transfer".to_string(), "ach,bacs".to_string()),
            ("wallet".to_string(), "paypal,venmo".to_string()),
        ]);
        let deserializer: MapDeserializer<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            ValueError,
        > = input_map.into_deserializer();
        let result = deserialize_hashmap::<'_, _, PaymentMethod, PaymentMethodType>(deserializer);
        let expected_result = HashMap::from([
            (
                PaymentMethod::BankTransfer,
                HashSet::from([PaymentMethodType::Ach, PaymentMethodType::Bacs]),
            ),
            (
                PaymentMethod::Wallet,
                HashSet::from([PaymentMethodType::Paypal, PaymentMethodType::Venmo]),
            ),
        ]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_result);
    }

    #[test]
    fn test_payment_method_and_payment_method_types_with_spaces() {
        use diesel_models::enums::{PaymentMethod, PaymentMethodType};

        let input_map: HashMap<String, String> = HashMap::from([
            (" bank_transfer ".to_string(), " ach , bacs ".to_string()),
            ("wallet ".to_string(), " paypal , pix , venmo ".to_string()),
        ]);
        let deserializer: MapDeserializer<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            ValueError,
        > = input_map.into_deserializer();
        let result = deserialize_hashmap::<'_, _, PaymentMethod, PaymentMethodType>(deserializer);
        let expected_result = HashMap::from([
            (
                PaymentMethod::BankTransfer,
                HashSet::from([PaymentMethodType::Ach, PaymentMethodType::Bacs]),
            ),
            (
                PaymentMethod::Wallet,
                HashSet::from([
                    PaymentMethodType::Paypal,
                    PaymentMethodType::Pix,
                    PaymentMethodType::Venmo,
                ]),
            ),
        ]);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_result);
    }

    #[test]
    fn test_payment_method_deserializer_error() {
        use diesel_models::enums::{PaymentMethod, PaymentMethodType};

        let input_map: HashMap<String, String> = HashMap::from([
            ("unknown".to_string(), "ach,bacs".to_string()),
            ("wallet".to_string(), "paypal,unknown".to_string()),
        ]);
        let deserializer: MapDeserializer<
            '_,
            std::collections::hash_map::IntoIter<String, String>,
            ValueError,
        > = input_map.into_deserializer();
        let result = deserialize_hashmap::<'_, _, PaymentMethod, PaymentMethodType>(deserializer);

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod hashset_deserialization_test {
    #![allow(clippy::unwrap_used)]
    use std::collections::HashSet;

    use serde::de::{
        value::{Error as ValueError, StrDeserializer},
        IntoDeserializer,
    };

    use super::deserialize_hashset;

    #[test]
    fn test_payment_method_hashset_deserializer() {
        use diesel_models::enums::PaymentMethod;

        let deserializer: StrDeserializer<'_, ValueError> = "wallet,card".into_deserializer();
        let payment_methods = deserialize_hashset::<'_, _, PaymentMethod>(deserializer);
        let expected_payment_methods = HashSet::from([PaymentMethod::Wallet, PaymentMethod::Card]);

        assert!(payment_methods.is_ok());
        assert_eq!(payment_methods.unwrap(), expected_payment_methods);
    }

    #[test]
    fn test_payment_method_hashset_deserializer_with_spaces() {
        use diesel_models::enums::PaymentMethod;

        let deserializer: StrDeserializer<'_, ValueError> =
            "wallet, card, bank_debit".into_deserializer();
        let payment_methods = deserialize_hashset::<'_, _, PaymentMethod>(deserializer);
        let expected_payment_methods = HashSet::from([
            PaymentMethod::Wallet,
            PaymentMethod::Card,
            PaymentMethod::BankDebit,
        ]);

        assert!(payment_methods.is_ok());
        assert_eq!(payment_methods.unwrap(), expected_payment_methods);
    }

    #[test]
    fn test_payment_method_hashset_deserializer_error() {
        use diesel_models::enums::PaymentMethod;

        let deserializer: StrDeserializer<'_, ValueError> =
            "wallet, card, unknown".into_deserializer();
        let payment_methods = deserialize_hashset::<'_, _, PaymentMethod>(deserializer);

        assert!(payment_methods.is_err());
    }
}
