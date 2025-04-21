# ClickToPaySessionResponse


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

## Example

```python
from hyperswitch.models.click_to_pay_session_response import ClickToPaySessionResponse

# TODO update the JSON string below
json = "{}"
# create an instance of ClickToPaySessionResponse from a JSON string
click_to_pay_session_response_instance = ClickToPaySessionResponse.from_json(json)
# print the JSON string representation of the object
print(ClickToPaySessionResponse.to_json())

# convert the object into a dict
click_to_pay_session_response_dict = click_to_pay_session_response_instance.to_dict()
# create an instance of ClickToPaySessionResponse from a dict
click_to_pay_session_response_from_dict = ClickToPaySessionResponse.from_dict(click_to_pay_session_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


