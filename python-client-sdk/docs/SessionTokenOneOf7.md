# SessionTokenOneOf7


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**dpa_id** | **str** |  | 
**dpa_name** | **str** |  | 
**locale** | **str** |  | 
**card_brands** | **List[str]** |  | 
**acquirer_bin** | **str** |  | 
**acquirer_merchant_id** | **str** |  | 
**merchant_category_code** | **str** |  | 
**merchant_country_code** | **str** |  | 
**transaction_amount** | **str** |  | 
**transaction_currency_code** | [**Currency**](Currency.md) |  | 
**phone_number** | **str** |  | [optional] 
**email** | **str** |  | [optional] 
**phone_country_code** | **str** |  | [optional] 
**provider** | [**CtpServiceProvider**](CtpServiceProvider.md) |  | [optional] 
**dpa_client_id** | **str** |  | [optional] 
**wallet_name** | **str** |  | 

## Example

```python
from hyperswitch.models.session_token_one_of7 import SessionTokenOneOf7

# TODO update the JSON string below
json = "{}"
# create an instance of SessionTokenOneOf7 from a JSON string
session_token_one_of7_instance = SessionTokenOneOf7.from_json(json)
# print the JSON string representation of the object
print(SessionTokenOneOf7.to_json())

# convert the object into a dict
session_token_one_of7_dict = session_token_one_of7_instance.to_dict()
# create an instance of SessionTokenOneOf7 from a dict
session_token_one_of7_from_dict = SessionTokenOneOf7.from_dict(session_token_one_of7_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


