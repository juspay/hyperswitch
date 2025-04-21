# SessionTokenOneOf6


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**client_id** | **str** | Paze Client ID | 
**client_name** | **str** | Client Name to be displayed on the Paze screen | 
**client_profile_id** | **str** | Paze Client Profile ID | 
**transaction_currency_code** | [**Currency**](Currency.md) |  | 
**transaction_amount** | **str** | The transaction amount | 
**email_address** | **str** | Email Address | [optional] 
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of6 import SessionTokenOneOf6

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf6 from a JSON string
session_token_one_of6_instance = SessionTokenOneOf6.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf6.to_json())

# convert the object into a dict
session_token_one_of6_dict = session_token_one_of6_instance.to_dict()
# create an instance of SessionTokenOneOf6 from a dict
session_token_one_of6_from_dict = SessionTokenOneOf6.from_dict(session_token_one_of6_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


