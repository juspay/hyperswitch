# ThirdPartySdkSessionResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**secrets** | [**SecretInfoToInitiateSdk**](SecretInfoToInitiateSdk.md) |  | 

## Example

```python
from hyperswitch.models.third_party_sdk_session_response import ThirdPartySdkSessionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ThirdPartySdkSessionResponse from a JSON string
third_party_sdk_session_response_instance = ThirdPartySdkSessionResponse.from_json(json)
# print the JSON string representation of the object
print(ThirdPartySdkSessionResponse.to_json())

# convert the object into a dict
third_party_sdk_session_response_dict = third_party_sdk_session_response_instance.to_dict()
# create an instance of ThirdPartySdkSessionResponse from a dict
third_party_sdk_session_response_from_dict = ThirdPartySdkSessionResponse.from_dict(third_party_sdk_session_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


