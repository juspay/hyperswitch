crate::id_type!(
    InvoiceId,
    " A type for invoice_id that can be used for invoice ids"
);

crate::impl_id_type_methods!(InvoiceId, "invoice_id");

// This is to display the `InvoiceId` as InvoiceId(subs)
crate::impl_debug_id_type!(InvoiceId);
crate::impl_try_from_cow_str_id_type!(InvoiceId, "invoice_id");

crate::impl_generate_id_id_type!(InvoiceId, "invoice");
crate::impl_serializable_secret_id_type!(InvoiceId);
crate::impl_queryable_id_type!(InvoiceId);
crate::impl_to_sql_from_sql_id_type!(InvoiceId);

impl crate::events::ApiEventMetric for InvoiceId {
    fn get_api_event_type(&self) -> Option<crate::events::ApiEventsType> {
        Some(crate::events::ApiEventsType::Invoice)
    }
}
