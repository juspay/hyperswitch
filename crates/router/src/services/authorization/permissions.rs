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
            scopes: [Read],
            entities: [Merchant]
        },
    ]
}
