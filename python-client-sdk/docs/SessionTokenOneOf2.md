# SessionTokenOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**session_token** | **str** | The session token for Klarna | 
**session_id** | **str** | The identifier for the session | 
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of2 import SessionTokenOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf2 from a JSON string
session_token_one_of2_instance = SessionTokenOneOf2.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf2.to_json())

# convert the object into a dict
session_token_one_of2_dict = session_token_one_of2_instance.to_dict()
# create an instance of SessionTokenOneOf2 from a dict
session_token_one_of2_from_dict = SessionTokenOneOf2.from_dict(session_token_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


