# CreateApiKeyResponse

The response body for creating an API Key.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**key_id** | **str** | The identifier for the API Key. | 
**merchant_id** | **str** | The identifier for the Merchant Account. | 
**name** | **str** | The unique name for the API Key to help you identify it. | 
**description** | **str** | The description to provide more context about the API Key. | [optional] 
**api_key** | **str** | The plaintext API Key used for server-side API access. Ensure you store the API Key securely as you will not be able to see it again. | 
**created** | **datetime** | The time at which the API Key was created. | 
**expiration** | [**ApiKeyExpiration**](ApiKeyExpiration.md) |  | 

## Example

```python
from hyperswitch.models.create_api_key_response import CreateApiKeyResponse

# TODO update the JSON string below
json = "{}"
# create an instance of CreateApiKeyResponse from a JSON string
create_api_key_response_instance = CreateApiKeyResponse.from_json(json)
# print the JSON string representation of the object
print(CreateApiKeyResponse.to_json())

# convert the object into a dict
create_api_key_response_dict = create_api_key_response_instance.to_dict()
# create an instance of CreateApiKeyResponse from a dict
create_api_key_response_from_dict = CreateApiKeyResponse.from_dict(create_api_key_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


