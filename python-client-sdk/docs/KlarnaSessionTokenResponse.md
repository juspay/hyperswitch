# KlarnaSessionTokenResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**session_token** | **str** | The session token for Klarna | 
**session_id** | **str** | The identifier for the session | 

## Example

```python
from hyperswitch.models.klarna_session_token_response import KlarnaSessionTokenResponse

# TODO update the JSON string below
json = "{}"
# create an instance of KlarnaSessionTokenResponse from a JSON string
klarna_session_token_response_instance = KlarnaSessionTokenResponse.from_json(json)
# print the JSON string representation of the object
print(KlarnaSessionTokenResponse.to_json())

# convert the object into a dict
klarna_session_token_response_dict = klarna_session_token_response_instance.to_dict()
# create an instance of KlarnaSessionTokenResponse from a dict
klarna_session_token_response_from_dict = KlarnaSessionTokenResponse.from_dict(klarna_session_token_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


