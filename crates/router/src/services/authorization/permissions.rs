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
        Subscription: {
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
            entities: [Profile, Merchant]
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
        RevenueRecovery: {
            scopes: [Read],
            entities: [Profile]
        },
        InternalConnector: {
            scopes: [Write],
            entities: [Merchant]
        },
        Theme: {
            scopes: [Read,Write],
            entities: [Organization]
        }
    ]
}

pub fn get_resource_name(resource: Resource, entity_type: EntityType) -> Option<&'static str> {
    match (resource, entity_type) {
        (Resource::Payment, _) => Some("Payments"),
        (Resource::Refund, _) => Some("Refunds"),
        (Resource::Dispute, _) => Some("Disputes"),
        (Resource::Mandate, _) => Some("Mandates"),
        (Resource::Customer, _) => Some("Customers"),
        (Resource::Payout, _) => Some("Payouts"),
        (Resource::ApiKey, _) => Some("Api Keys"),
        (Resource::Connector, _) => {
            Some("Payment Processors, Payout Processors, Fraud & Risk Managers")
        }
        (Resource::Routing, _) => Some("Routing"),
        (Resource::Subscription, _) => Some("Subscription"),
        (Resource::RevenueRecovery, _) => Some("Revenue Recovery"),
        (Resource::ThreeDsDecisionManager, _) => Some("3DS Decision Manager"),
        (Resource::SurchargeDecisionManager, _) => Some("Surcharge Decision Manager"),
        (Resource::Analytics, _) => Some("Analytics"),
        (Resource::Report, _) => Some("Operation Reports"),
        (Resource::User, _) => Some("Users"),
        (Resource::WebhookEvent, _) => Some("Webhook Events"),
        (Resource::ReconUpload, _) => Some("Reconciliation File Upload"),
        (Resource::RunRecon, _) => Some("Run Reconciliation Process"),
        (Resource::ReconConfig, _) => Some("Reconciliation Configurations"),
        (Resource::ReconToken, _) => Some("Generate & Verify Reconciliation Token"),
        (Resource::ReconFiles, _) => Some("Reconciliation Process Manager"),
        (Resource::ReconReports, _) => Some("Reconciliation Reports"),
        (Resource::ReconAndSettlementAnalytics, _) => Some("Reconciliation Analytics"),
        (Resource::Account, EntityType::Profile) => Some("Business Profile Account"),
        (Resource::Account, EntityType::Merchant) => Some("Merchant Account"),
        (Resource::Account, EntityType::Organization) => Some("Organization Account"),
        (Resource::Account, EntityType::Tenant) => Some("Tenant Account"),
        (Resource::Theme, _) => Some("Themes"),
        (Resource::InternalConnector, _) => None,
    }
}

pub fn get_scope_name(scope: PermissionScope) -> &'static str {
    match scope {
        PermissionScope::Read => "View",
        PermissionScope::Write => "View and Manage",
    }
}

pub fn filter_resources_by_entity_type(
    resources: Vec<Resource>,
    entity_type: EntityType,
) -> Option<Vec<Resource>> {
    let filtered: Vec<Resource> = resources
        .into_iter()
        .filter(|res| res.entities().iter().any(|entity| entity <= &entity_type))
        .collect();

    (!filtered.is_empty()).then_some(filtered)
}
