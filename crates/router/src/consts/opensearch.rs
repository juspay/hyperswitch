use crate::services::authorization::permissions::Permission;
use api_models::analytics::search::SearchIndex;

pub const OPENSEARCH_INDEX_PERMISSIONS: &[(SearchIndex, &[Permission])] = &[
    (
        SearchIndex::PaymentAttempts,
        &[Permission::PaymentRead, Permission::PaymentWrite],
    ),
    (
        SearchIndex::PaymentIntents,
        &[Permission::PaymentRead, Permission::PaymentWrite],
    ),
    (
        SearchIndex::Refunds,
        &[Permission::RefundRead, Permission::RefundWrite],
    ),
    (
        SearchIndex::Disputes,
        &[Permission::DisputeRead, Permission::DisputeWrite],
    ),
];
