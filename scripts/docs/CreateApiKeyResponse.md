# CreateApiKeyResponse

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**key_id** | **String** | The identifier for the API Key. | 
**merchant_id** | **String** | The identifier for the Merchant Account. | 
**name** | **String** | The unique name for the API Key to help you identify it. | 
**description** | Option<**String**> | The description to provide more context about the API Key. | [optional]
**api_key** | **String** | The plaintext API Key used for server-side API access. Ensure you store the API Key securely as you will not be able to see it again. | 
**created** | **String** | The time at which the API Key was created. | 
**expiration** | [**crate::models::ApiKeyExpiration**](ApiKeyExpiration.md) |  | 

[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


