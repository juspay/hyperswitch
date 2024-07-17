// impl api::PaymentIncrementalAuthorization for Helcim {}
// impl api::ConnectorCustomer for Helcim {}
// impl api::PaymentsPreProcessing for Helcim {}
// impl api::PaymentReject for Helcim {}
// impl api::PaymentApprove for Helcim {}
use hyperswitch_domain_models::{
    router_flow_types::payments::{
        Approve, AuthorizeSessionToken, CompleteAuthorize, CreateConnectorCustomer,
        IncrementalAuthorization, PreProcessing, Reject,
    },
    router_request_types::{
        AuthorizeSessionTokenData, CompleteAuthorizeData, ConnectorCustomerData,
        PaymentsApproveData, PaymentsIncrementalAuthorizationData, PaymentsPreProcessingData,
        PaymentsRejectData,
    },
    router_response_types::PaymentsResponseData,
};
use hyperswitch_interfaces::api::{
    payments::{
        ConnectorCustomer, PaymentApprove, PaymentAuthorizeSessionToken,
        PaymentIncrementalAuthorization, PaymentReject, PaymentsCompleteAuthorize,
        PaymentsPreProcessing,
    },
    ConnectorIntegration,
};

macro_rules! default_imp_for_authorize_session_token {
    ($($path:ident::$connector:ident),*) => {
        $( impl PaymentAuthorizeSessionToken for $path::$connector {}
            impl
            ConnectorIntegration<
                AuthorizeSessionToken,
                AuthorizeSessionTokenData,
                PaymentsResponseData
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_authorize_session_token!(connectors::Helcim);

use crate::connectors;
macro_rules! default_imp_for_complete_authorize {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentsCompleteAuthorize for $path::$connector {}
            impl
            ConnectorIntegration<
            CompleteAuthorize,
            CompleteAuthorizeData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_complete_authorize!(connectors::Helcim);

macro_rules! default_imp_for_incremental_authorization {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentIncrementalAuthorization for $path::$connector {}
            impl
            ConnectorIntegration<
            IncrementalAuthorization,
            PaymentsIncrementalAuthorizationData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_incremental_authorization!(connectors::Helcim);

macro_rules! default_imp_for_create_customer {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl ConnectorCustomer for $path::$connector {}
            impl
            ConnectorIntegration<
            CreateConnectorCustomer,
            ConnectorCustomerData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_create_customer!(connectors::Helcim);

macro_rules! default_imp_for_pre_processing_steps{
    ($($path:ident::$connector:ident),*)=> {
        $(
            impl PaymentsPreProcessing for $path::$connector {}
            impl
            ConnectorIntegration<
            PreProcessing,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_pre_processing_steps!(connectors::Helcim);

macro_rules! default_imp_for_approve {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentApprove for $path::$connector {}
            impl
            ConnectorIntegration<
            Approve,
            PaymentsApproveData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_approve!(connectors::Helcim);

macro_rules! default_imp_for_reject {
    ($($path:ident::$connector:ident),*) => {
        $(
            impl PaymentReject for $path::$connector {}
            impl
            ConnectorIntegration<
            Reject,
            PaymentsRejectData,
            PaymentsResponseData,
        > for $path::$connector
        {}
    )*
    };
}

default_imp_for_reject!(connectors::Helcim);
