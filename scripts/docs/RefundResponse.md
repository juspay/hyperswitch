# RefundResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**refund_id** | **String** | The identifier for refund | 
**payment_id** | **String** | The identifier for payment | 
**amount** | **i64** | The refund amount, which should be less than or equal to the total payment amount. Amount for the payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR denomination etc | 
**currency** | **String** | The three-letter ISO currency code | 
**reason** | Option<**String**> | An arbitrary string attached to the object. Often useful for displaying to users and your customer support executive | [optional]
**status** | [**crate::models::RefundStatus**](RefundStatus.md) |  | 
**metadata** | Option<[**serde_json::Value**](.md)> | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object | [optional]
**error_message** | Option<**String**> | The error message | [optional]
**error_code** | Option<**String**> | The code for the error | [optional]
**created_at** | Option<**String**> | The timestamp at which refund is created | [optional]
**updated_at** | Option<**String**> | The timestamp at which refund is updated | [optional]
**connector** | **String** | The connector used for the refund and the corresponding payment | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


