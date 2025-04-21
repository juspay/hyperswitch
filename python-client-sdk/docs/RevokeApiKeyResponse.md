# RevokeApiKeyResponse

The response body for revoking an API Key.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The identifier for the Merchant Account. | 
**key_id** | **str** | The identifier for the API Key. | 
**revoked** | **bool** | Indicates whether the API key was revoked or not. | 

## Example

```python
from hyperswitch.models.revoke_api_key_response import RevokeApiKeyResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RevokeApiKeyResponse from a JSON string
revoke_api_key_response_instance = RevokeApiKeyResponse.from_json(json)
# print the JSON string representation of the object
print(RevokeApiKeyResponse.to_json())

# convert the object into a dict
revoke_api_key_response_dict = revoke_api_key_response_instance.to_dict()
# create an instance of RevokeApiKeyResponse from a dict
revoke_api_key_response_from_dict = RevokeApiKeyResponse.from_dict(revoke_api_key_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


