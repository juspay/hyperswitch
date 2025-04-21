# RetrieveApiKeyResponse

The response body for retrieving an API Key.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**key_id** | **str** | The identifier for the API Key. | 
**merchant_id** | **str** | The identifier for the Merchant Account. | 
**name** | **str** | The unique name for the API Key to help you identify it. | 
**description** | **str** | The description to provide more context about the API Key. | [optional] 
**prefix** | **str** | The first few characters of the plaintext API Key to help you identify it. | 
**created** | **datetime** | The time at which the API Key was created. | 
**expiration** | [**ApiKeyExpiration**](ApiKeyExpiration.md) |  | 

## Example

```python
from hyperswitch.models.retrieve_api_key_response import RetrieveApiKeyResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RetrieveApiKeyResponse from a JSON string
retrieve_api_key_response_instance = RetrieveApiKeyResponse.from_json(json)
# print the JSON string representation of the object
print(RetrieveApiKeyResponse.to_json())

# convert the object into a dict
retrieve_api_key_response_dict = retrieve_api_key_response_instance.to_dict()
# create an instance of RetrieveApiKeyResponse from a dict
retrieve_api_key_response_from_dict = RetrieveApiKeyResponse.from_dict(retrieve_api_key_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


