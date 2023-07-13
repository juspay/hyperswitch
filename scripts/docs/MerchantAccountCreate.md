# MerchantAccountCreate

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **String** | The identifier for the Merchant Account | 
**merchant_name** | Option<**String**> | Name of the Merchant Account | [optional]
**merchant_details** | Option<[**crate::models::MerchantDetails**](MerchantDetails.md)> |  | [optional]
**return_url** | Option<**String**> | The URL to redirect after the completion of the operation | [optional]
**webhook_details** | Option<[**crate::models::WebhookDetails**](WebhookDetails.md)> |  | [optional]
**routing_algorithm** | Option<[**serde_json::Value**](.md)> | The routing algorithm to be used for routing payments to desired connectors | [optional]
**sub_merchants_enabled** | Option<**bool**> | A boolean value to indicate if the merchant is a sub-merchant under a master or a parent merchant. By default, its value is false. | [optional][default to false]
**parent_merchant_id** | Option<**String**> | Refers to the Parent Merchant ID if the merchant being created is a sub-merchant | [optional]
**enable_payment_response_hash** | Option<**bool**> | A boolean value to indicate if payment response hash needs to be enabled | [optional][default to false]
**payment_response_hash_key** | Option<**String**> | Refers to the hash key used for payment response | [optional]
**redirect_to_merchant_with_http_post** | Option<**bool**> | A boolean value to indicate if redirect to merchant with http post needs to be enabled | [optional][default to false]
**metadata** | Option<[**serde_json::Value**](.md)> | You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object. | [optional]
**publishable_key** | Option<**String**> | API key that will be used for server side API access | [optional]
**locker_id** | Option<**String**> | An identifier for the vault used to store payment method information. | [optional]
**primary_business_details** | Option<[**crate::models::PrimaryBusinessDetails**](PrimaryBusinessDetails.md)> |  | [optional]
**frm_routing_algorithm** | Option<[**serde_json::Value**](.md)> | The frm routing algorithm to be used for routing payments to desired FRM's | [optional]
**intent_fulfillment_time** | Option<**i32**> | Will be used to expire client secret after certain amount of time to be supplied in seconds (900) for 15 mins | [optional]

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


