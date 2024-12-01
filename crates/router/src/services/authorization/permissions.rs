use common_enums::{EntityType, PermissionScope, Resource};
use router_derive::generate_permissions;

generate_permissions! {
    permissions: [
        Payment: {
            scopes: [Read, Write],
            entities: [Profile, Merchant]
        },
        Refund: {
            scopes: [Read, Write],
            entities: [Profile, Merchant]
        },
        Dispute: {
            scopes: [Read, Write],
            entities: [Profile, Merchant]
        },
        Mandate: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        Customer: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        Payout: {
            scopes: [Read],
            entities: [Profile, Merchant]
        },
        ApiKey: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        Account: {
            scopes: [Read, Write],
            entities: [Profile, Merchant, Organization]
        },
        Connector: {
            scopes: [Read, Write],
            entities: [Profile, Merchant]
        },
        Routing: {
            scopes: [Read, Write],
            entities: [Profile, Merchant]
        },
        ThreeDsDecisionManager: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        SurchargeDecisionManager: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        Analytics: {
            scopes: [Read],
            entities: [Profile, Merchant, Organization]
        },
        Report: {
            scopes: [Read],
            entities: [Profile, Merchant, Organization]
        },
        User: {
            scopes: [Read, Write],
            entities: [Profile, Merchant]
        },
        WebhookEvent: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        Recon: {
            scopes: [Write],
            entities: [Merchant]
        },
    ]
}

pub fn get_resource_name(resource: Resource, entity_type: EntityType) -> &'static str {
    match (resource, entity_type) {
        (Resource::Payment, _) => "Payments",
        (Resource::Refund, _) => "Refunds",
        (Resource::Dispute, _) => "Disputes",
        (Resource::Mandate, _) => "Mandates",
        (Resource::Customer, _) => "Customers",
        (Resource::Payout, _) => "Payouts",
        (Resource::ApiKey, _) => "Api Keys",
        (Resource::Connector, _) => "Payment Processors, Payout Processors, Fraud & Risk Managers",
        (Resource::Routing, _) => "Routing",
        (Resource::ThreeDsDecisionManager, _) => "3DS Decision Manager",
        (Resource::SurchargeDecisionManager, _) => "Surcharge Decision Manager",
        (Resource::Analytics, _) => "Analytics",
        (Resource::Report, _) => "Operation Reports",
        (Resource::User, _) => "Users",
        (Resource::WebhookEvent, _) => "Webhook Events",
        (Resource::Recon, _) => "Reconciliation Reports",
        (Resource::Account, EntityType::Profile) => "Business Profile Account",
        (Resource::Account, EntityType::Merchant) => "Merchant Account",
        (Resource::Account, EntityType::Organization) => "Organization Account",
    }
}

pub fn get_scope_name(scope: PermissionScope) -> &'static str {
    match scope {
        PermissionScope::Read => "View",
        PermissionScope::Write => "View and Manage",
    }
}
