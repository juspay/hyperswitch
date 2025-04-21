# SessionTokenOneOf3


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | **str** | Name of the connector | 
**session_token** | **str** | The session token for PayPal | 
**sdk_next_action** | [**SdkNextAction**](SdkNextAction.md) |  | 
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of3 import SessionTokenOneOf3

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf3 from a JSON string
session_token_one_of3_instance = SessionTokenOneOf3.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf3.to_json())

# convert the object into a dict
session_token_one_of3_dict = session_token_one_of3_instance.to_dict()
# create an instance of SessionTokenOneOf3 from a dict
session_token_one_of3_from_dict = SessionTokenOneOf3.from_dict(session_token_one_of3_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


