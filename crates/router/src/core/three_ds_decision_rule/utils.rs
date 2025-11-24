use api_models::three_ds_decision_rule as api_threedsecure;
use common_types::three_ds_decision_rule_engine::ThreeDSDecision;
use euclid::backend::inputs as dsl_inputs;

use crate::{consts::PSD2_COUNTRIES, types::transformers::ForeignFrom};

// function to apply PSD2 validations to the decision
pub fn apply_psd2_validations_during_execute(
    decision: ThreeDSDecision,
    request: &api_models::three_ds_decision_rule::ThreeDsDecisionRuleExecuteRequest,
) -> ThreeDSDecision {
    let issuer_in_psd2 = request
        .issuer
        .as_ref()
        .and_then(|issuer| issuer.country)
        .map(|country| PSD2_COUNTRIES.contains(&country))
        .unwrap_or(false);
    let acquirer_in_psd2 = request
        .acquirer
        .as_ref()
        .and_then(|acquirer| acquirer.country)
        .map(|country| PSD2_COUNTRIES.contains(&country))
        .unwrap_or(false);
    if issuer_in_psd2 && acquirer_in_psd2 {
        // If both issuer and acquirer are in PSD2 region
        match decision {
            // If the decision is to enforce no 3DS, override it to enforce 3DS
            ThreeDSDecision::NoThreeDs => ThreeDSDecision::ChallengeRequested,
            _ => decision,
        }
    } else {
        // If PSD2 doesn't apply, exemptions cannot be applied
        match decision {
            ThreeDSDecision::NoThreeDs => ThreeDSDecision::NoThreeDs,
            // For all other decisions (including exemptions), enforce challenge as exemptions are only valid in PSD2 regions
            _ => ThreeDSDecision::ChallengeRequested,
        }
    }
}

impl ForeignFrom<api_threedsecure::PaymentData> for dsl_inputs::PaymentInput {
    fn foreign_from(request_payment_data: api_threedsecure::PaymentData) -> Self {
        Self {
            amount: request_payment_data.amount,
            currency: request_payment_data.currency,
            authentication_type: None,
            capture_method: None,
            business_country: None,
            billing_country: None,
            business_label: None,
            setup_future_usage: None,
            card_bin: None,
            extended_card_bin: None,
        }
    }
}

impl ForeignFrom<Option<api_threedsecure::PaymentMethodMetaData>>
    for dsl_inputs::PaymentMethodInput
{
    fn foreign_from(
        request_payment_method_metadata: Option<api_threedsecure::PaymentMethodMetaData>,
    ) -> Self {
        Self {
            payment_method: None,
            payment_method_type: None,
            card_network: request_payment_method_metadata.and_then(|pm| pm.card_network),
        }
    }
}

impl ForeignFrom<api_threedsecure::CustomerDeviceData> for dsl_inputs::CustomerDeviceDataInput {
    fn foreign_from(request_customer_device_data: api_threedsecure::CustomerDeviceData) -> Self {
        Self {
            platform: request_customer_device_data.platform,
            device_type: request_customer_device_data.device_type,
            display_size: request_customer_device_data.display_size,
        }
    }
}

impl ForeignFrom<api_threedsecure::IssuerData> for dsl_inputs::IssuerDataInput {
    fn foreign_from(request_issuer_data: api_threedsecure::IssuerData) -> Self {
        Self {
            name: request_issuer_data.name,
            country: request_issuer_data.country,
        }
    }
}

impl ForeignFrom<api_threedsecure::AcquirerData> for dsl_inputs::AcquirerDataInput {
    fn foreign_from(request_acquirer_data: api_threedsecure::AcquirerData) -> Self {
        Self {
            country: request_acquirer_data.country,
            fraud_rate: request_acquirer_data.fraud_rate,
        }
    }
}

impl ForeignFrom<api_threedsecure::ThreeDsDecisionRuleExecuteRequest> for dsl_inputs::BackendInput {
    fn foreign_from(request: api_threedsecure::ThreeDsDecisionRuleExecuteRequest) -> Self {
        Self {
            metadata: None,
            payment: dsl_inputs::PaymentInput::foreign_from(request.payment),
            payment_method: dsl_inputs::PaymentMethodInput::foreign_from(request.payment_method),
            mandate: dsl_inputs::MandateData {
                mandate_acceptance_type: None,
                mandate_type: None,
                payment_type: None,
            },
            acquirer_data: request.acquirer.map(ForeignFrom::foreign_from),
            customer_device_data: request.customer_device.map(ForeignFrom::foreign_from),
            issuer_data: request.issuer.map(ForeignFrom::foreign_from),
        }
    }
}
