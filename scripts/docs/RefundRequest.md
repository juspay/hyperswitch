# RefundRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**refund_id** | Option<**String**> | Unique Identifier for the Refund. This is to ensure idempotency for multiple partial refund initiated against the same payment. If the identifiers is not defined by the merchant, this filed shall be auto generated and provide in the API response. It is recommended to generate uuid(v4) as the refund_id. | [optional]
**payment_id** | **String** | Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc. If not provided, this will default to the full payment amount | 
**merchant_id** | Option<**String**> | The identifier for the Merchant Account | [optional]
**amount** | Option<**i64**> | Total amount for which the refund is to be initiated. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc., If not provided, this will default to the full payment amount | [optional]
**reason** | Option<**String**> | An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive | [optional]
**refund_type** | Option<[**crate::models::RefundType**](RefundType.md)> |  | [optional]
**metadata** | Option<[**serde_json::Value**](.md)> | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional]
**merchant_connector_details** | Option<[**crate::models::MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


