# PaymentsCaptureRequest

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_id** | Option<**String**> | The unique identifier for the payment | [optional]
**merchant_id** | Option<**String**> | The unique identifier for the merchant | [optional]
**amount_to_capture** | Option<**i64**> | The Amount to be captured/ debited from the user's payment method. | [optional]
**refund_uncaptured_amount** | Option<**bool**> | Decider to refund the uncaptured amount | [optional]
**statement_descriptor_suffix** | Option<**String**> | Provides information about a card payment that customers see on their statements. | [optional]
**statement_descriptor_prefix** | Option<**String**> | Concatenated with the statement descriptor suffix that’s set on the account to form the complete statement descriptor. | [optional]
**merchant_connector_details** | Option<[**crate::models::MerchantConnectorDetailsWrap**](MerchantConnectorDetailsWrap.md)> |  | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


