pub use hyperswitch_domain_models::{
    errors::api_error_response, router_request_types::subscriptions as subscription_request_types,
};

pub const X_PROFILE_ID: &str = "X-Profile-Id";
pub const X_TENANT_ID: &str = "x-tenant-id";
pub const X_MERCHANT_ID: &str = "X-Merchant-Id";
pub const X_INTERNAL_API_KEY: &str = "X-Internal-Api-Key";

pub trait ForeignFrom<F> {
    fn foreign_from(from: F) -> Self;
}

/// Trait for converting from one foreign type to another
pub trait ForeignTryFrom<F>: Sized {
    /// Custom error for conversion failure
    type Error;
    /// Convert from a foreign type to the current type and return an error if the conversion fails
    fn foreign_try_from(from: F) -> Result<Self, Self::Error>;
}

pub trait StorageErrorExt<T, E> {
    #[track_caller]
    fn to_not_found_response(self, not_found_response: E) -> error_stack::Result<T, E>;

    #[track_caller]
    fn to_duplicate_response(self, duplicate_response: E) -> error_stack::Result<T, E>;
}

impl<T> StorageErrorExt<T, api_error_response::ApiErrorResponse>
    for error_stack::Result<T, storage_impl::StorageError>
{
    #[track_caller]
    fn to_not_found_response(
        self,
        not_found_response: api_error_response::ApiErrorResponse,
    ) -> error_stack::Result<T, api_error_response::ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                storage_impl::StorageError::ValueNotFound(_) => not_found_response,
                storage_impl::StorageError::CustomerRedacted => {
                    api_error_response::ApiErrorResponse::CustomerRedacted
                }
                _ => api_error_response::ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }

    #[track_caller]
    fn to_duplicate_response(
        self,
        duplicate_response: api_error_response::ApiErrorResponse,
    ) -> error_stack::Result<T, api_error_response::ApiErrorResponse> {
        self.map_err(|err| {
            let new_err = match err.current_context() {
                storage_impl::StorageError::DuplicateValue { .. } => duplicate_response,
                _ => api_error_response::ApiErrorResponse::InternalServerError,
            };
            err.change_context(new_err)
        })
    }
}
/// Extension trait for adding addons to subscription items
pub trait AddonsExt {
    fn add_to_subscription_items(
        self,
        subscription_items: Vec<subscription_request_types::SubscriptionItem>,
    ) -> Vec<subscription_request_types::SubscriptionItem>;
}

impl AddonsExt for Option<Vec<api_models::subscription::AddonsDetails>> {
    fn add_to_subscription_items(
        self,
        mut subscription_items: Vec<subscription_request_types::SubscriptionItem>,
    ) -> Vec<subscription_request_types::SubscriptionItem> {
        if let Some(addon_list) = self {
            for addon in addon_list {
                subscription_items.push(subscription_request_types::SubscriptionItem {
                    item_price_id: addon.item_price_id.clone(),
                    quantity: addon.quantity,
                });
            }
        }
        subscription_items
    }
}
