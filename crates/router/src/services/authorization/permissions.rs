use strum::Display;

#[derive(PartialEq, Display, Clone, Debug)]
pub enum Permission {
    PaymentRead,
    PaymentWrite,
    RefundRead,
    RefundWrite,
    ApiKeyRead,
    ApiKeyWrite,
    MerchantAccountRead,
    MerchantAccountWrite,
    MerchantConnectorAccountRead,
    MerchantConnectorAccountWrite,
    ForexRead,
    RoutingRead,
    RoutingWrite,
    DisputeRead,
    DisputeWrite,
    MandateRead,
    MandateWrite,
    CustomerRead,
    CustomerWrite,
    FileRead,
    FileWrite,
    Analytics,
    ThreeDsDecisionManagerWrite,
    ThreeDsDecisionManagerRead,
    SurchargeDecisionManagerWrite,
    SurchargeDecisionManagerRead,
    UsersRead,
    UsersWrite,
    MerchantAccountCreate,
}

impl Permission {
    pub fn get_permission_description(&self) -> Option<&'static str> {
        match self {
            Self::PaymentRead => Some("View all payments"),
            Self::PaymentWrite => Some("Create payment, download payments data"),
            Self::RefundRead => Some("View all refunds"),
            Self::RefundWrite => Some("Create refund, download refunds data"),
            Self::ApiKeyRead => Some("View API keys (masked generated for the system"),
            Self::ApiKeyWrite => Some("Create and update API keys"),
            Self::MerchantAccountRead => Some("View merchant account details"),
            Self::MerchantAccountWrite => {
                Some("Update merchant account details, configure webhooks, manage api keys")
            }
            Self::MerchantConnectorAccountRead => Some("View connectors configured"),
            Self::MerchantConnectorAccountWrite => {
                Some("Create, update, verify and delete connector configurations")
            }
            Self::ForexRead => Some("Query Forex data"),
            Self::RoutingRead => Some("View routing configuration"),
            Self::RoutingWrite => Some("Create and activate routing configurations"),
            Self::DisputeRead => Some("View disputes"),
            Self::DisputeWrite => Some("Create and update disputes"),
            Self::MandateRead => Some("View mandates"),
            Self::MandateWrite => Some("Create and update mandates"),
            Self::CustomerRead => Some("View customers"),
            Self::CustomerWrite => Some("Create, update and delete customers"),
            Self::FileRead => Some("View files"),
            Self::FileWrite => Some("Create, update and delete files"),
            Self::Analytics => Some("Access to analytics module"),
            Self::ThreeDsDecisionManagerWrite => Some("Create and update 3DS decision rules"),
            Self::ThreeDsDecisionManagerRead => {
                Some("View all 3DS decision rules configured for a merchant")
            }
            Self::SurchargeDecisionManagerWrite => {
                Some("Create and update the surcharge decision rules")
            }
            Self::SurchargeDecisionManagerRead => Some("View all the surcharge decision rules"),
            Self::UsersRead => Some("View all the users for a merchant"),
            Self::UsersWrite => Some("Invite users, assign and update roles"),
            Self::MerchantAccountCreate => None,
        }
    }
}
