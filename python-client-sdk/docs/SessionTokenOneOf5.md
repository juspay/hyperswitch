# SessionTokenOneOf5


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**open_banking_session_token** | **str** | The session token for OpenBanking Connectors | 
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of5 import SessionTokenOneOf5

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf5 from a JSON string
session_token_one_of5_instance = SessionTokenOneOf5.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf5.to_json())

# convert the object into a dict
session_token_one_of5_dict = session_token_one_of5_instance.to_dict()
# create an instance of SessionTokenOneOf5 from a dict
session_token_one_of5_from_dict = SessionTokenOneOf5.from_dict(session_token_one_of5_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


