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
            entities: [Profile, Merchant, Organization, Tenant]
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
            entities: [Merchant, Profile]
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
        ReconToken: {
            scopes: [Read],
            entities: [Merchant]
        },
        ReconFiles: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        ReconAndSettlementAnalytics: {
            scopes: [Read],
            entities: [Merchant]
        },
        ReconUpload: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        ReconReports: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        RunRecon: {
            scopes: [Read, Write],
            entities: [Merchant]
        },
        ReconConfig: {
            scopes: [Read, Write],
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
        (Resource::ReconUpload, _) => "Reconciliation File Upload",
        (Resource::RunRecon, _) => "Run Reconciliation Process",
        (Resource::ReconConfig, _) => "Reconciliation Configurations",
        (Resource::ReconToken, _) => "Generate & Verify Reconciliation Token",
        (Resource::ReconFiles, _) => "Reconciliation Process Manager",
        (Resource::ReconReports, _) => "Reconciliation Reports",
        (Resource::ReconAndSettlementAnalytics, _) => "Reconciliation Analytics",
        (Resource::Account, EntityType::Profile) => "Business Profile Account",
        (Resource::Account, EntityType::Merchant) => "Merchant Account",
        (Resource::Account, EntityType::Organization) => "Organization Account",
        (Resource::Account, EntityType::Tenant) => "Tenant Account",
    }
}

pub fn get_scope_name(scope: PermissionScope) -> &'static str {
    match scope {
        PermissionScope::Read => "View",
        PermissionScope::Write => "View and Manage",
    }
}
