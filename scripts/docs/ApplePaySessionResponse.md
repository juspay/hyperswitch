# ApplePaySessionResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**secrets** | [**crate::models::SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md) |  | 
**epoch_timestamp** | **i64** | Timestamp at which session is requested | 
**expires_at** | **i64** | Timestamp at which session expires | 
**merchant_session_identifier** | **String** | The identifier for the merchant session | 
**nonce** | **String** | Apple pay generated unique ID (UUID) value | 
**merchant_identifier** | **String** | The identifier for the merchant | 
**domain_name** | **String** | The domain name of the merchant which is registered in Apple Pay | 
**display_name** | **String** | The name to be displayed on Apple Pay button | 
**signature** | **String** | A string which represents the properties of a payment | 
**operational_analytics_identifier** | **String** | The identifier for the operational analytics | 
**retries** | **i32** | The number of retries to get the session response | 
**psp_id** | **String** | The identifier for the connector transaction | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


