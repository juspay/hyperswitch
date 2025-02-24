use std::str::FromStr;

use api_models::{
    admin as admin_api, enums as api_enums, payment_methods::RequestPaymentMethodTypes,
    refunds::MinorUnit,
};
use euclid::{
    dirval,
    frontend::{ast, dir},
    types::{NumValue, NumValueRefinement},
};
use hyperswitch_constraint_graph as cgraph;
use strum::IntoEnumIterator;

use crate::{error::KgraphError, transformers::IntoDirValue, types as kgraph_types};

pub const DOMAIN_IDENTIFIER: &str = "payment_methods_enabled_for_merchantconnectoraccount";

#[cfg(feature = "v1")]
fn get_dir_value_payment_method(
    from: api_enums::PaymentMethodType,
) -> Result<dir::DirValue, KgraphError> {
    match from {
        api_enums::PaymentMethodType::AmazonPay => Ok(dirval!(WalletType = AmazonPay)),
        api_enums::PaymentMethodType::Credit => Ok(dirval!(CardType = Credit)),
        api_enums::PaymentMethodType::Debit => Ok(dirval!(CardType = Debit)),
        api_enums::PaymentMethodType::Giropay => Ok(dirval!(BankRedirectType = Giropay)),
        api_enums::PaymentMethodType::Ideal => Ok(dirval!(BankRedirectType = Ideal)),
        api_enums::PaymentMethodType::Sofort => Ok(dirval!(BankRedirectType = Sofort)),
        api_enums::PaymentMethodType::Eps => Ok(dirval!(BankRedirectType = Eps)),
        api_enums::PaymentMethodType::Eft => Ok(dirval!(BankRedirectType = Eft)),
        api_enums::PaymentMethodType::Klarna => Ok(dirval!(PayLaterType = Klarna)),
        api_enums::PaymentMethodType::Affirm => Ok(dirval!(PayLaterType = Affirm)),
        api_enums::PaymentMethodType::AfterpayClearpay => {
            Ok(dirval!(PayLaterType = AfterpayClearpay))
        }
        api_enums::PaymentMethodType::GooglePay => Ok(dirval!(WalletType = GooglePay)),
        api_enums::PaymentMethodType::ApplePay => Ok(dirval!(WalletType = ApplePay)),
        api_enums::PaymentMethodType::Paypal => Ok(dirval!(WalletType = Paypal)),
        api_enums::PaymentMethodType::CryptoCurrency => Ok(dirval!(CryptoType = CryptoCurrency)),
        api_enums::PaymentMethodType::Ach => Ok(dirval!(BankDebitType = Ach)),

        api_enums::PaymentMethodType::Bacs => Ok(dirval!(BankDebitType = Bacs)),

        api_enums::PaymentMethodType::Becs => Ok(dirval!(BankDebitType = Becs)),
        api_enums::PaymentMethodType::Sepa => Ok(dirval!(BankDebitType = Sepa)),

        api_enums::PaymentMethodType::AliPay => Ok(dirval!(WalletType = AliPay)),
        api_enums::PaymentMethodType::AliPayHk => Ok(dirval!(WalletType = AliPayHk)),
        api_enums::PaymentMethodType::BancontactCard => {
            Ok(dirval!(BankRedirectType = BancontactCard))
        }
        api_enums::PaymentMethodType::Blik => Ok(dirval!(BankRedirectType = Blik)),
        api_enums::PaymentMethodType::MbWay => Ok(dirval!(WalletType = MbWay)),
        api_enums::PaymentMethodType::MobilePay => Ok(dirval!(WalletType = MobilePay)),
        api_enums::PaymentMethodType::Cashapp => Ok(dirval!(WalletType = Cashapp)),
        api_enums::PaymentMethodType::Multibanco => Ok(dirval!(BankTransferType = Multibanco)),
        api_enums::PaymentMethodType::Pix => Ok(dirval!(BankTransferType = Pix)),
        api_enums::PaymentMethodType::Pse => Ok(dirval!(BankTransferType = Pse)),
        api_enums::PaymentMethodType::Interac => Ok(dirval!(BankRedirectType = Interac)),
        api_enums::PaymentMethodType::OnlineBankingCzechRepublic => {
            Ok(dirval!(BankRedirectType = OnlineBankingCzechRepublic))
        }
        api_enums::PaymentMethodType::OnlineBankingFinland => {
            Ok(dirval!(BankRedirectType = OnlineBankingFinland))
        }
        api_enums::PaymentMethodType::OnlineBankingPoland => {
            Ok(dirval!(BankRedirectType = OnlineBankingPoland))
        }
        api_enums::PaymentMethodType::OnlineBankingSlovakia => {
            Ok(dirval!(BankRedirectType = OnlineBankingSlovakia))
        }
        api_enums::PaymentMethodType::Swish => Ok(dirval!(WalletType = Swish)),
        api_enums::PaymentMethodType::Trustly => Ok(dirval!(BankRedirectType = Trustly)),
        api_enums::PaymentMethodType::Bizum => Ok(dirval!(BankRedirectType = Bizum)),

        api_enums::PaymentMethodType::PayBright => Ok(dirval!(PayLaterType = PayBright)),
        api_enums::PaymentMethodType::Walley => Ok(dirval!(PayLaterType = Walley)),
        api_enums::PaymentMethodType::Przelewy24 => Ok(dirval!(BankRedirectType = Przelewy24)),
        api_enums::PaymentMethodType::WeChatPay => Ok(dirval!(WalletType = WeChatPay)),

        api_enums::PaymentMethodType::ClassicReward => Ok(dirval!(RewardType = ClassicReward)),
        api_enums::PaymentMethodType::Evoucher => Ok(dirval!(RewardType = Evoucher)),
        api_enums::PaymentMethodType::SamsungPay => Ok(dirval!(WalletType = SamsungPay)),
        api_enums::PaymentMethodType::GoPay => Ok(dirval!(WalletType = GoPay)),
        api_enums::PaymentMethodType::KakaoPay => Ok(dirval!(WalletType = KakaoPay)),
        api_enums::PaymentMethodType::Twint => Ok(dirval!(WalletType = Twint)),
        api_enums::PaymentMethodType::Gcash => Ok(dirval!(WalletType = Gcash)),
        api_enums::PaymentMethodType::Vipps => Ok(dirval!(WalletType = Vipps)),
        api_enums::PaymentMethodType::Momo => Ok(dirval!(WalletType = Momo)),
        api_enums::PaymentMethodType::Alma => Ok(dirval!(PayLaterType = Alma)),
        api_enums::PaymentMethodType::Dana => Ok(dirval!(WalletType = Dana)),
        api_enums::PaymentMethodType::OnlineBankingFpx => {
            Ok(dirval!(BankRedirectType = OnlineBankingFpx))
        }
        api_enums::PaymentMethodType::OnlineBankingThailand => {
            Ok(dirval!(BankRedirectType = OnlineBankingThailand))
        }
        api_enums::PaymentMethodType::LocalBankRedirect => {
            Ok(dirval!(BankRedirectType = LocalBankRedirect))
        }
        api_enums::PaymentMethodType::TouchNGo => Ok(dirval!(WalletType = TouchNGo)),
        api_enums::PaymentMethodType::Atome => Ok(dirval!(PayLaterType = Atome)),
        api_enums::PaymentMethodType::Boleto => Ok(dirval!(VoucherType = Boleto)),
        api_enums::PaymentMethodType::Efecty => Ok(dirval!(VoucherType = Efecty)),
        api_enums::PaymentMethodType::PagoEfectivo => Ok(dirval!(VoucherType = PagoEfectivo)),
        api_enums::PaymentMethodType::RedCompra => Ok(dirval!(VoucherType = RedCompra)),
        api_enums::PaymentMethodType::RedPagos => Ok(dirval!(VoucherType = RedPagos)),
        api_enums::PaymentMethodType::Alfamart => Ok(dirval!(VoucherType = Alfamart)),
        api_enums::PaymentMethodType::BcaBankTransfer => {
            Ok(dirval!(BankTransferType = BcaBankTransfer))
        }
        api_enums::PaymentMethodType::BniVa => Ok(dirval!(BankTransferType = BniVa)),
        api_enums::PaymentMethodType::BriVa => Ok(dirval!(BankTransferType = BriVa)),
        api_enums::PaymentMethodType::CimbVa => Ok(dirval!(BankTransferType = CimbVa)),
        api_enums::PaymentMethodType::DanamonVa => Ok(dirval!(BankTransferType = DanamonVa)),
        api_enums::PaymentMethodType::Indomaret => Ok(dirval!(VoucherType = Indomaret)),
        api_enums::PaymentMethodType::MandiriVa => Ok(dirval!(BankTransferType = MandiriVa)),
        api_enums::PaymentMethodType::LocalBankTransfer => {
            Ok(dirval!(BankTransferType = LocalBankTransfer))
        }
        api_enums::PaymentMethodType::PermataBankTransfer => {
            Ok(dirval!(BankTransferType = PermataBankTransfer))
        }
        api_enums::PaymentMethodType::PaySafeCard => Ok(dirval!(GiftCardType = PaySafeCard)),
        api_enums::PaymentMethodType::SevenEleven => Ok(dirval!(VoucherType = SevenEleven)),
        api_enums::PaymentMethodType::Lawson => Ok(dirval!(VoucherType = Lawson)),
        api_enums::PaymentMethodType::MiniStop => Ok(dirval!(VoucherType = MiniStop)),
        api_enums::PaymentMethodType::FamilyMart => Ok(dirval!(VoucherType = FamilyMart)),
        api_enums::PaymentMethodType::Seicomart => Ok(dirval!(VoucherType = Seicomart)),
        api_enums::PaymentMethodType::PayEasy => Ok(dirval!(VoucherType = PayEasy)),
        api_enums::PaymentMethodType::Givex => Ok(dirval!(GiftCardType = Givex)),
        api_enums::PaymentMethodType::Benefit => Ok(dirval!(CardRedirectType = Benefit)),
        api_enums::PaymentMethodType::Knet => Ok(dirval!(CardRedirectType = Knet)),
        api_enums::PaymentMethodType::OpenBankingUk => {
            Ok(dirval!(BankRedirectType = OpenBankingUk))
        }
        api_enums::PaymentMethodType::MomoAtm => Ok(dirval!(CardRedirectType = MomoAtm)),
        api_enums::PaymentMethodType::Oxxo => Ok(dirval!(VoucherType = Oxxo)),
        api_enums::PaymentMethodType::CardRedirect => Ok(dirval!(CardRedirectType = CardRedirect)),
        api_enums::PaymentMethodType::Venmo => Ok(dirval!(WalletType = Venmo)),
        api_enums::PaymentMethodType::UpiIntent => Ok(dirval!(UpiType = UpiIntent)),
        api_enums::PaymentMethodType::UpiCollect => Ok(dirval!(UpiType = UpiCollect)),
        api_enums::PaymentMethodType::Mifinity => Ok(dirval!(WalletType = Mifinity)),
        api_enums::PaymentMethodType::Fps => Ok(dirval!(RealTimePaymentType = Fps)),
        api_enums::PaymentMethodType::DuitNow => Ok(dirval!(RealTimePaymentType = DuitNow)),
        api_enums::PaymentMethodType::PromptPay => Ok(dirval!(RealTimePaymentType = PromptPay)),
        api_enums::PaymentMethodType::VietQr => Ok(dirval!(RealTimePaymentType = VietQr)),
        api_enums::PaymentMethodType::OpenBankingPIS => {
            Ok(dirval!(OpenBankingType = OpenBankingPIS))
        }
        api_enums::PaymentMethodType::Paze => Ok(dirval!(WalletType = Paze)),
        api_enums::PaymentMethodType::DirectCarrierBilling => {
            Ok(dirval!(MobilePaymentType = DirectCarrierBilling))
        }
    }
}

#[cfg(feature = "v1")]
fn compile_request_pm_types(
    builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
    pm_types: RequestPaymentMethodTypes,
    pm: api_enums::PaymentMethod,
) -> Result<cgraph::NodeId, KgraphError> {
    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

    let pmt_info = "PaymentMethodType";
    let pmt_id = builder.make_value_node(
        (pm_types.payment_method_type, pm)
            .into_dir_value()
            .map(Into::into)?,
        Some(pmt_info),
        None::<()>,
    );
    agg_nodes.push((
        pmt_id,
        cgraph::Relation::Positive,
        match pm_types.payment_method_type {
            api_enums::PaymentMethodType::Credit | api_enums::PaymentMethodType::Debit => {
                cgraph::Strength::Weak
            }

            _ => cgraph::Strength::Strong,
        },
    ));

    if let Some(card_networks) = pm_types.card_networks {
        if !card_networks.is_empty() {
            let dir_vals: Vec<dir::DirValue> = card_networks
                .into_iter()
                .map(IntoDirValue::into_dir_value)
                .collect::<Result<_, _>>()?;

            let card_network_info = "Card Networks";
            let card_network_id = builder
                .make_in_aggregator(dir_vals, Some(card_network_info), None::<()>)
                .map_err(KgraphError::GraphConstructionError)?;

            agg_nodes.push((
                card_network_id,
                cgraph::Relation::Positive,
                cgraph::Strength::Weak,
            ));
        }
    }

    let currencies_data = pm_types
        .accepted_currencies
        .and_then(|accepted_currencies| match accepted_currencies {
            admin_api::AcceptedCurrencies::EnableOnly(curr) if !curr.is_empty() => Some((
                curr.into_iter()
                    .map(IntoDirValue::into_dir_value)
                    .collect::<Result<_, _>>()
                    .ok()?,
                cgraph::Relation::Positive,
            )),

            admin_api::AcceptedCurrencies::DisableOnly(curr) if !curr.is_empty() => Some((
                curr.into_iter()
                    .map(IntoDirValue::into_dir_value)
                    .collect::<Result<_, _>>()
                    .ok()?,
                cgraph::Relation::Negative,
            )),

            _ => None,
        });

    if let Some((currencies, relation)) = currencies_data {
        let accepted_currencies_info = "Accepted Currencies";
        let accepted_currencies_id = builder
            .make_in_aggregator(currencies, Some(accepted_currencies_info), None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;

        agg_nodes.push((accepted_currencies_id, relation, cgraph::Strength::Strong));
    }

    let mut amount_nodes = Vec::with_capacity(2);

    if let Some(min_amt) = pm_types.minimum_amount {
        let num_val = NumValue {
            number: min_amt,
            refinement: Some(NumValueRefinement::GreaterThanEqual),
        };

        let min_amt_info = "Minimum Amount";
        let min_amt_id = builder.make_value_node(
            dir::DirValue::PaymentAmount(num_val).into(),
            Some(min_amt_info),
            None::<()>,
        );

        amount_nodes.push(min_amt_id);
    }

    if let Some(max_amt) = pm_types.maximum_amount {
        let num_val = NumValue {
            number: max_amt,
            refinement: Some(NumValueRefinement::LessThanEqual),
        };

        let max_amt_info = "Maximum Amount";
        let max_amt_id = builder.make_value_node(
            dir::DirValue::PaymentAmount(num_val).into(),
            Some(max_amt_info),
            None::<()>,
        );

        amount_nodes.push(max_amt_id);
    }

    if !amount_nodes.is_empty() {
        let zero_num_val = NumValue {
            number: MinorUnit::zero(),
            refinement: None,
        };

        let zero_amt_id = builder.make_value_node(
            dir::DirValue::PaymentAmount(zero_num_val).into(),
            Some("zero_amount"),
            None::<()>,
        );

        let or_node_neighbor_id = if amount_nodes.len() == 1 {
            amount_nodes
                .first()
                .copied()
                .ok_or(KgraphError::IndexingError)?
        } else {
            let nodes = amount_nodes
                .iter()
                .copied()
                .map(|node_id| {
                    (
                        node_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    )
                })
                .collect::<Vec<_>>();

            builder
                .make_all_aggregator(
                    &nodes,
                    Some("amount_constraint_aggregator"),
                    None::<()>,
                    None,
                )
                .map_err(KgraphError::GraphConstructionError)?
        };

        let any_aggregator = builder
            .make_any_aggregator(
                &[
                    (
                        zero_amt_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ),
                    (
                        or_node_neighbor_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ),
                ],
                Some("zero_plus_limits_amount_aggregator"),
                None::<()>,
                None,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        agg_nodes.push((
            any_aggregator,
            cgraph::Relation::Positive,
            cgraph::Strength::Strong,
        ));
    }

    let pmt_all_aggregator_info = "All Aggregator for PaymentMethodType";
    builder
        .make_all_aggregator(&agg_nodes, Some(pmt_all_aggregator_info), None::<()>, None)
        .map_err(KgraphError::GraphConstructionError)
}

#[cfg(feature = "v1")]
fn compile_payment_method_enabled(
    builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
    enabled: admin_api::PaymentMethodsEnabled,
) -> Result<Option<cgraph::NodeId>, KgraphError> {
    let agg_id = if !enabled
        .payment_method_types
        .as_ref()
        .map(|v| v.is_empty())
        .unwrap_or(true)
    {
        let pm_info = "PaymentMethod";
        let pm_id = builder.make_value_node(
            enabled.payment_method.into_dir_value().map(Into::into)?,
            Some(pm_info),
            None::<()>,
        );

        let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

        if let Some(pm_types) = enabled.payment_method_types {
            for pm_type in pm_types {
                let node_id = compile_request_pm_types(builder, pm_type, enabled.payment_method)?;
                agg_nodes.push((
                    node_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                ));
            }
        }

        let any_aggregator_info = "Any aggregation for PaymentMethodsType";
        let pm_type_agg_id = builder
            .make_any_aggregator(&agg_nodes, Some(any_aggregator_info), None::<()>, None)
            .map_err(KgraphError::GraphConstructionError)?;

        let all_aggregator_info = "All aggregation for PaymentMethod";
        let enabled_pm_agg_id = builder
            .make_all_aggregator(
                &[
                    (pm_id, cgraph::Relation::Positive, cgraph::Strength::Strong),
                    (
                        pm_type_agg_id,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Strong,
                    ),
                ],
                Some(all_aggregator_info),
                None::<()>,
                None,
            )
            .map_err(KgraphError::GraphConstructionError)?;

        Some(enabled_pm_agg_id)
    } else {
        None
    };

    Ok(agg_id)
}

macro_rules! collect_global_variants {
    ($parent_enum:ident) => {
        &mut dir::enums::$parent_enum::iter()
            .map(dir::DirValue::$parent_enum)
            .collect::<Vec<_>>()
    };
}

#[cfg(feature = "v1")]
fn global_vec_pmt(
    enabled_pmt: Vec<dir::DirValue>,
    builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
) -> Vec<cgraph::NodeId> {
    let mut global_vector: Vec<dir::DirValue> = Vec::new();

    global_vector.append(collect_global_variants!(PayLaterType));
    global_vector.append(collect_global_variants!(WalletType));
    global_vector.append(collect_global_variants!(BankRedirectType));
    global_vector.append(collect_global_variants!(BankDebitType));
    global_vector.append(collect_global_variants!(CryptoType));
    global_vector.append(collect_global_variants!(RewardType));
    global_vector.append(collect_global_variants!(RealTimePaymentType));
    global_vector.append(collect_global_variants!(UpiType));
    global_vector.append(collect_global_variants!(VoucherType));
    global_vector.append(collect_global_variants!(GiftCardType));
    global_vector.append(collect_global_variants!(BankTransferType));
    global_vector.append(collect_global_variants!(CardRedirectType));
    global_vector.append(collect_global_variants!(OpenBankingType));
    global_vector.append(collect_global_variants!(MobilePaymentType));
    global_vector.push(dir::DirValue::PaymentMethod(
        dir::enums::PaymentMethod::Card,
    ));

    let global_vector = global_vector
        .into_iter()
        .filter(|global_value| !enabled_pmt.contains(global_value))
        .collect::<Vec<_>>();

    global_vector
        .into_iter()
        .map(|dir_v| {
            builder.make_value_node(
                cgraph::NodeValue::Value(dir_v),
                Some("Payment Method Type"),
                None::<()>,
            )
        })
        .collect::<Vec<_>>()
}

fn compile_graph_for_countries_and_currencies(
    builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
    config: &kgraph_types::CurrencyCountryFlowFilter,
    payment_method_type_node: cgraph::NodeId,
) -> Result<cgraph::NodeId, KgraphError> {
    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    agg_nodes.push((
        payment_method_type_node,
        cgraph::Relation::Positive,
        cgraph::Strength::Normal,
    ));
    if let Some(country) = config.country.clone() {
        let node_country = country
            .into_iter()
            .map(|country| dir::DirValue::BillingCountry(api_enums::Country::from_alpha2(country)))
            .collect();
        let country_agg = builder
            .make_in_aggregator(node_country, Some("Configs for Country"), None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;
        agg_nodes.push((
            country_agg,
            cgraph::Relation::Positive,
            cgraph::Strength::Weak,
        ))
    }

    if let Some(currency) = config.currency.clone() {
        let node_currency = currency
            .into_iter()
            .map(IntoDirValue::into_dir_value)
            .collect::<Result<Vec<_>, _>>()?;
        let currency_agg = builder
            .make_in_aggregator(node_currency, Some("Configs for Currency"), None::<()>)
            .map_err(KgraphError::GraphConstructionError)?;
        agg_nodes.push((
            currency_agg,
            cgraph::Relation::Positive,
            cgraph::Strength::Normal,
        ))
    }
    if let Some(capture_method) = config
        .not_available_flows
        .and_then(|naf| naf.capture_method)
    {
        let make_capture_node = builder.make_value_node(
            cgraph::NodeValue::Value(dir::DirValue::CaptureMethod(capture_method)),
            Some("Configs for CaptureMethod"),
            None::<()>,
        );
        agg_nodes.push((
            make_capture_node,
            cgraph::Relation::Negative,
            cgraph::Strength::Normal,
        ))
    }

    builder
        .make_all_aggregator(
            &agg_nodes,
            Some("Country & Currency Configs With Payment Method Type"),
            None::<()>,
            None,
        )
        .map_err(KgraphError::GraphConstructionError)
}

#[cfg(feature = "v1")]
fn compile_config_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
    config: &kgraph_types::CountryCurrencyFilter,
    connector: api_enums::RoutableConnectors,
) -> Result<cgraph::NodeId, KgraphError> {
    let mut agg_node_id: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();
    let mut pmt_enabled: Vec<dir::DirValue> = Vec::new();
    if let Some(pmt) = config
        .connector_configs
        .get(&connector)
        .or(config.default_configs.as_ref())
        .map(|inner| inner.0.clone())
    {
        for pm_filter_key in pmt {
            match pm_filter_key {
                (kgraph_types::PaymentMethodFilterKey::PaymentMethodType(pm), filter) => {
                    let dir_val_pm = get_dir_value_payment_method(pm)?;

                    let pm_node = if pm == api_enums::PaymentMethodType::Credit
                        || pm == api_enums::PaymentMethodType::Debit
                    {
                        pmt_enabled
                            .push(dir::DirValue::PaymentMethod(api_enums::PaymentMethod::Card));
                        builder.make_value_node(
                            cgraph::NodeValue::Value(dir::DirValue::PaymentMethod(
                                dir::enums::PaymentMethod::Card,
                            )),
                            Some("PaymentMethod"),
                            None::<()>,
                        )
                    } else {
                        pmt_enabled.push(dir_val_pm.clone());
                        builder.make_value_node(
                            cgraph::NodeValue::Value(dir_val_pm),
                            Some("PaymentMethodType"),
                            None::<()>,
                        )
                    };

                    let node_config =
                        compile_graph_for_countries_and_currencies(builder, &filter, pm_node)?;

                    agg_node_id.push((
                        node_config,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Normal,
                    ));
                }
                (kgraph_types::PaymentMethodFilterKey::CardNetwork(cn), filter) => {
                    let dir_val_cn = cn.clone().into_dir_value()?;
                    pmt_enabled.push(dir_val_cn);
                    let cn_node = builder.make_value_node(
                        cn.clone().into_dir_value().map(Into::into)?,
                        Some("CardNetwork"),
                        None::<()>,
                    );
                    let node_config =
                        compile_graph_for_countries_and_currencies(builder, &filter, cn_node)?;

                    agg_node_id.push((
                        node_config,
                        cgraph::Relation::Positive,
                        cgraph::Strength::Normal,
                    ));
                }
            }
        }
    }
    let global_vector_pmt: Vec<cgraph::NodeId> = global_vec_pmt(pmt_enabled, builder);
    let any_agg_pmt: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = global_vector_pmt
        .into_iter()
        .map(|node| (node, cgraph::Relation::Positive, cgraph::Strength::Normal))
        .collect::<Vec<_>>();
    let any_agg_node = builder
        .make_any_aggregator(
            &any_agg_pmt,
            Some("Any Aggregator For Payment Method Types"),
            None::<()>,
            None,
        )
        .map_err(KgraphError::GraphConstructionError)?;

    agg_node_id.push((
        any_agg_node,
        cgraph::Relation::Positive,
        cgraph::Strength::Normal,
    ));

    builder
        .make_any_aggregator(&agg_node_id, Some("Configs"), None::<()>, None)
        .map_err(KgraphError::GraphConstructionError)
}

#[cfg(feature = "v1")]
fn compile_merchant_connector_graph(
    builder: &mut cgraph::ConstraintGraphBuilder<dir::DirValue>,
    mca: admin_api::MerchantConnectorResponse,
    config: &kgraph_types::CountryCurrencyFilter,
) -> Result<(), KgraphError> {
    let connector = common_enums::RoutableConnectors::from_str(&mca.connector_name)
        .map_err(|_| KgraphError::InvalidConnectorName(mca.connector_name.clone()))?;

    let mut agg_nodes: Vec<(cgraph::NodeId, cgraph::Relation, cgraph::Strength)> = Vec::new();

    if let Some(pms_enabled) = mca.payment_methods_enabled.clone() {
        for pm_enabled in pms_enabled {
            let maybe_pm_enabled_id = compile_payment_method_enabled(builder, pm_enabled)?;
            if let Some(pm_enabled_id) = maybe_pm_enabled_id {
                agg_nodes.push((
                    pm_enabled_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Strong,
                ));
            }
        }
    }

    let aggregator_info = "Available Payment methods for connector";
    let pms_enabled_agg_id = builder
        .make_any_aggregator(&agg_nodes, Some(aggregator_info), None::<()>, None)
        .map_err(KgraphError::GraphConstructionError)?;

    let config_info = "Config for respective PaymentMethodType for the connector";

    let config_enabled_agg_id = compile_config_graph(builder, config, connector)?;

    let domain_level_node_id = builder
        .make_all_aggregator(
            &[
                (
                    config_enabled_agg_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Normal,
                ),
                (
                    pms_enabled_agg_id,
                    cgraph::Relation::Positive,
                    cgraph::Strength::Normal,
                ),
            ],
            Some(config_info),
            None::<()>,
            None,
        )
        .map_err(KgraphError::GraphConstructionError)?;
    let connector_dir_val = dir::DirValue::Connector(Box::new(ast::ConnectorChoice { connector }));

    let connector_info = "Connector";
    let connector_node_id =
        builder.make_value_node(connector_dir_val.into(), Some(connector_info), None::<()>);

    builder
        .make_edge(
            domain_level_node_id,
            connector_node_id,
            cgraph::Strength::Normal,
            cgraph::Relation::Positive,
            None::<cgraph::DomainId>,
        )
        .map_err(KgraphError::GraphConstructionError)?;

    Ok(())
}

#[cfg(feature = "v1")]
pub fn make_mca_graph(
    accts: Vec<admin_api::MerchantConnectorResponse>,
    config: &kgraph_types::CountryCurrencyFilter,
) -> Result<cgraph::ConstraintGraph<dir::DirValue>, KgraphError> {
    let mut builder = cgraph::ConstraintGraphBuilder::new();
    let _domain = builder.make_domain(
        DOMAIN_IDENTIFIER.to_string(),
        "Payment methods enabled for MerchantConnectorAccount",
    );
    for acct in accts {
        compile_merchant_connector_graph(&mut builder, acct, config)?;
    }

    Ok(builder.build())
}

#[cfg(feature = "v1")]
#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use std::collections::{HashMap, HashSet};

    use api_models::enums as api_enums;
    use euclid::{
        dirval,
        dssa::graph::{AnalysisContext, CgraphExt},
    };
    use hyperswitch_constraint_graph::{ConstraintGraph, CycleCheck, Memoization};

    use super::*;
    use crate::types as kgraph_types;

    fn build_test_data() -> ConstraintGraph<dir::DirValue> {
        use api_models::{admin::*, payment_methods::*};
        let profile_id = common_utils::generate_profile_id_of_default_length();

        // #[cfg(feature = "v2")]
        // let stripe_account = MerchantConnectorResponse {
        //     connector_type: api_enums::ConnectorType::FizOperations,
        //     connector_name: "stripe".to_string(),
        //     id: common_utils::generate_merchant_connector_account_id_of_default_length(),
        //     connector_label: Some("something".to_string()),
        //     connector_account_details: masking::Secret::new(serde_json::json!({})),
        //     disabled: None,
        //     metadata: None,
        //     payment_methods_enabled: Some(vec![PaymentMethodsEnabled {
        //         payment_method: api_enums::PaymentMethod::Card,
        //         payment_method_types: Some(vec![
        //             RequestPaymentMethodTypes {
        //                 payment_method_type: api_enums::PaymentMethodType::Credit,
        //                 payment_experience: None,
        //                 card_networks: Some(vec![
        //                     api_enums::CardNetwork::Visa,
        //                     api_enums::CardNetwork::Mastercard,
        //                 ]),
        //                 accepted_currencies: Some(AcceptedCurrencies::EnableOnly(vec![
        //                     api_enums::Currency::INR,
        //                 ])),
        //                 accepted_countries: None,
        //                 minimum_amount: Some(MinorUnit::new(10)),
        //                 maximum_amount: Some(MinorUnit::new(1000)),
        //                 recurring_enabled: true,
        //                 installment_payment_enabled: true,
        //             },
        //             RequestPaymentMethodTypes {
        //                 payment_method_type: api_enums::PaymentMethodType::Debit,
        //                 payment_experience: None,
        //                 card_networks: Some(vec![
        //                     api_enums::CardNetwork::Maestro,
        //                     api_enums::CardNetwork::JCB,
        //                 ]),
        //                 accepted_currencies: Some(AcceptedCurrencies::EnableOnly(vec![
        //                     api_enums::Currency::GBP,
        //                 ])),
        //                 accepted_countries: None,
        //                 minimum_amount: Some(MinorUnit::new(10)),
        //                 maximum_amount: Some(MinorUnit::new(1000)),
        //                 recurring_enabled: true,
        //                 installment_payment_enabled: true,
        //             },
        //         ]),
        //     }]),
        //     frm_configs: None,
        //     connector_webhook_details: None,
        //     profile_id,
        //     applepay_verified_domains: None,
        //     pm_auth_config: None,
        //     status: api_enums::ConnectorStatus::Inactive,
        //     additional_merchant_data: None,
        //     connector_wallets_details: None,
        // };
        #[cfg(feature = "v1")]
        let stripe_account = MerchantConnectorResponse {
            connector_type: api_enums::ConnectorType::FizOperations,
            connector_name: "stripe".to_string(),
            merchant_connector_id:
                common_utils::generate_merchant_connector_account_id_of_default_length(),
            business_country: Some(api_enums::CountryAlpha2::US),
            connector_label: Some("something".to_string()),
            business_label: Some("food".to_string()),
            business_sub_label: None,
            connector_account_details: masking::Secret::new(serde_json::json!({})),
            test_mode: None,
            disabled: None,
            metadata: None,
            payment_methods_enabled: Some(vec![PaymentMethodsEnabled {
                payment_method: api_enums::PaymentMethod::Card,
                payment_method_types: Some(vec![
                    RequestPaymentMethodTypes {
                        payment_method_type: api_enums::PaymentMethodType::Credit,
                        payment_experience: None,
                        card_networks: Some(vec![
                            api_enums::CardNetwork::Visa,
                            api_enums::CardNetwork::Mastercard,
                        ]),
                        accepted_currencies: Some(AcceptedCurrencies::EnableOnly(vec![
                            api_enums::Currency::INR,
                        ])),
                        accepted_countries: None,
                        minimum_amount: Some(MinorUnit::new(10)),
                        maximum_amount: Some(MinorUnit::new(1000)),
                        recurring_enabled: true,
                        installment_payment_enabled: true,
                    },
                    RequestPaymentMethodTypes {
                        payment_method_type: api_enums::PaymentMethodType::Debit,
                        payment_experience: None,
                        card_networks: Some(vec![
                            api_enums::CardNetwork::Maestro,
                            api_enums::CardNetwork::JCB,
                        ]),
                        accepted_currencies: Some(AcceptedCurrencies::EnableOnly(vec![
                            api_enums::Currency::GBP,
                        ])),
                        accepted_countries: None,
                        minimum_amount: Some(MinorUnit::new(10)),
                        maximum_amount: Some(MinorUnit::new(1000)),
                        recurring_enabled: true,
                        installment_payment_enabled: true,
                    },
                ]),
            }]),
            frm_configs: None,
            connector_webhook_details: None,
            profile_id,
            applepay_verified_domains: None,
            pm_auth_config: None,
            status: api_enums::ConnectorStatus::Inactive,
            additional_merchant_data: None,
            connector_wallets_details: None,
        };

        let config_map = kgraph_types::CountryCurrencyFilter {
            connector_configs: HashMap::from([(
                api_enums::RoutableConnectors::Stripe,
                kgraph_types::PaymentMethodFilters(HashMap::from([
                    (
                        kgraph_types::PaymentMethodFilterKey::PaymentMethodType(
                            api_enums::PaymentMethodType::Credit,
                        ),
                        kgraph_types::CurrencyCountryFlowFilter {
                            currency: Some(HashSet::from([
                                api_enums::Currency::INR,
                                api_enums::Currency::USD,
                            ])),
                            country: Some(HashSet::from([api_enums::CountryAlpha2::IN])),
                            not_available_flows: Some(kgraph_types::NotAvailableFlows {
                                capture_method: Some(api_enums::CaptureMethod::Manual),
                            }),
                        },
                    ),
                    (
                        kgraph_types::PaymentMethodFilterKey::PaymentMethodType(
                            api_enums::PaymentMethodType::Debit,
                        ),
                        kgraph_types::CurrencyCountryFlowFilter {
                            currency: Some(HashSet::from([
                                api_enums::Currency::GBP,
                                api_enums::Currency::PHP,
                            ])),
                            country: Some(HashSet::from([api_enums::CountryAlpha2::IN])),
                            not_available_flows: Some(kgraph_types::NotAvailableFlows {
                                capture_method: Some(api_enums::CaptureMethod::Manual),
                            }),
                        },
                    ),
                ])),
            )]),
            default_configs: None,
        };

        make_mca_graph(vec![stripe_account], &config_map).expect("Failed graph construction")
    }

    #[test]
    fn test_credit_card_success_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Credit),
                dirval!(CardNetwork = Visa),
                dirval!(PaymentCurrency = INR),
                dirval!(PaymentAmount = 101),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_debit_card_success_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Maestro),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok());
    }

    #[test]
    fn test_single_mismatch_failure_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Maestro),
                dirval!(PaymentCurrency = PHP),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_amount_mismatch_failure_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Visa),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 7),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_data_failure_case() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(PaymentMethod = Card),
                dirval!(CardType = Debit),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 7),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        //println!("{:#?}", result);
        //println!("{}", serde_json::to_string_pretty(&result).expect("Hello"));

        assert!(result.is_err());
    }

    #[test]
    fn test_incomplete_data_failure_case2() {
        let graph = build_test_data();

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &AnalysisContext::from_dir_values([
                dirval!(Connector = Stripe),
                dirval!(CardType = Debit),
                dirval!(CardNetwork = Visa),
                dirval!(PaymentCurrency = GBP),
                dirval!(PaymentAmount = 100),
            ]),
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        //println!("{:#?}", result);
        //println!("{}", serde_json::to_string_pretty(&result).expect("Hello"));

        assert!(result.is_err());
    }

    #[test]
    fn test_sandbox_applepay_bug_usecase() {
        let value = serde_json::json!([
            {
                "connector_type": "payment_processor",
                "connector_name": "bluesnap",
                "merchant_connector_id": "REDACTED",
                "status": "inactive",
                "connector_account_details": {
                    "auth_type": "BodyKey",
                    "api_key": "REDACTED",
                    "key1": "REDACTED"
                },
                "test_mode": true,
                "disabled": false,
                "payment_methods_enabled": [
                    {
                        "payment_method": "card",
                        "payment_method_types": [
                            {
                                "payment_method_type": "credit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            },
                            {
                                "payment_method_type": "debit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "Interac",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    },
                    {
                        "payment_method": "wallet",
                        "payment_method_types": [
                            {
                                "payment_method_type": "google_pay",
                                "payment_experience": "invoke_sdk_client",
                                "card_networks": null,
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    }
                ],
                "metadata": {},
                "business_country": "US",
                "business_label": "default",
                "business_sub_label": null,
                "frm_configs": null
            },
            {
                "connector_type": "payment_processor",
                "connector_name": "stripe",
                "merchant_connector_id": "REDACTED",
                "status": "inactive",
                "connector_account_details": {
                    "auth_type": "HeaderKey",
                    "api_key": "REDACTED"
                },
                "test_mode": true,
                "disabled": false,
                "payment_methods_enabled": [
                    {
                        "payment_method": "card",
                        "payment_method_types": [
                            {
                                "payment_method_type": "credit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            },
                            {
                                "payment_method_type": "debit",
                                "payment_experience": null,
                                "card_networks": [
                                    "Mastercard",
                                    "Visa",
                                    "Interac",
                                    "AmericanExpress",
                                    "JCB",
                                    "DinersClub",
                                    "Discover",
                                    "CartesBancaires",
                                    "UnionPay"
                                ],
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    },
                    {
                        "payment_method": "wallet",
                        "payment_method_types": [
                            {
                                "payment_method_type": "apple_pay",
                                "payment_experience": "invoke_sdk_client",
                                "card_networks": null,
                                "accepted_currencies": null,
                                "accepted_countries": null,
                                "minimum_amount": 1,
                                "maximum_amount": 68607706,
                                "recurring_enabled": true,
                                "installment_payment_enabled": true
                            }
                        ]
                    },
                    {
                        "payment_method": "pay_later",
                        "payment_method_types": []
                    }
                ],
                "metadata": {},
                "business_country": "US",
                "business_label": "default",
                "business_sub_label": null,
                "frm_configs": null
            }
        ]);

        let data: Vec<admin_api::MerchantConnectorResponse> =
            serde_json::from_value(value).expect("data");
        let config = kgraph_types::CountryCurrencyFilter {
            connector_configs: HashMap::new(),
            default_configs: None,
        };
        let graph = make_mca_graph(data, &config).expect("graph");
        let context = AnalysisContext::from_dir_values([
            dirval!(Connector = Stripe),
            dirval!(PaymentAmount = 212),
            dirval!(PaymentCurrency = ILS),
            dirval!(PaymentMethod = Wallet),
            dirval!(WalletType = ApplePay),
        ]);

        let result = graph.key_value_analysis(
            dirval!(Connector = Stripe),
            &context,
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );

        assert!(result.is_ok(), "stripe validation failed");

        let result = graph.key_value_analysis(
            dirval!(Connector = Bluesnap),
            &context,
            &mut Memoization::new(),
            &mut CycleCheck::new(),
            None,
        );
        assert!(result.is_err(), "bluesnap validation failed");
    }
}
