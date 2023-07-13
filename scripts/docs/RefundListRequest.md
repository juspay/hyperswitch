# RefundListRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | Option<**String**> | The identifier for the payment | [optional]
**limit** | Option<**i64**> | Limit on the number of objects to return | [optional]
**offset** | Option<**i64**> | The starting point within a list of objects | [optional]
**time_range** | Option<[**crate::models::TimeRange**](TimeRange.md)> |  | [optional]
**connector** | Option<**Vec<String>**> | The list of connectors to filter refunds list | [optional]
**currency** | Option<[**Vec<crate::models::Currency>**](Currency.md)> | The list of currencies to filter refunds list | [optional]
**refund_status** | Option<[**Vec<crate::models::RefundStatus>**](RefundStatus.md)> | The list of refund statuses to filter refunds list | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


