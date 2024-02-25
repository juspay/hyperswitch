use strum::Display;

#[derive(PartialEq, Display, Clone, Debug, Copy, Eq, Hash)]
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
    RoutingRead,
    RoutingWrite,
    DisputeRead,
    DisputeWrite,
    MandateRead,
    MandateWrite,
    CustomerRead,
    CustomerWrite,
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
    pub fn get_permission_description(&self) -> &'static str {
        match self {
            Self::PaymentRead => "View all payments",
            Self::PaymentWrite => "Create payment, download payments data",
            Self::RefundRead => "View all refunds",
            Self::RefundWrite => "Create refund, download refunds data",
            Self::ApiKeyRead => "View API keys (masked generated for the system",
            Self::ApiKeyWrite => "Create and update API keys",
            Self::MerchantAccountRead => "View merchant account details",
            Self::MerchantAccountWrite => {
                "Update merchant account details, configure webhooks, manage api keys"
            }
            Self::MerchantConnectorAccountRead => "View connectors configured",
            Self::MerchantConnectorAccountWrite => {
                "Create, update, verify and delete connector configurations"
            }
            Self::RoutingRead => "View routing configuration",
            Self::RoutingWrite => "Create and activate routing configurations",
            Self::DisputeRead => "View disputes",
            Self::DisputeWrite => "Create and update disputes",
            Self::MandateRead => "View mandates",
            Self::MandateWrite => "Create and update mandates",
            Self::CustomerRead => "View customers",
            Self::CustomerWrite => "Create, update and delete customers",
            Self::Analytics => "Access to analytics module",
            Self::ThreeDsDecisionManagerWrite => "Create and update 3DS decision rules",
            Self::ThreeDsDecisionManagerRead => {
                "View all 3DS decision rules configured for a merchant"
            }
            Self::SurchargeDecisionManagerWrite => "Create and update the surcharge decision rules",
            Self::SurchargeDecisionManagerRead => "View all the surcharge decision rules",
            Self::UsersRead => "View all the users for a merchant",
            Self::UsersWrite => "Invite users, assign and update roles",
            Self::MerchantAccountCreate => "Create merchant account",
        }
    }
}
