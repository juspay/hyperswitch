# PaypalAdditionalData

Masked payout method details for paypal wallet payout method

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **str** | Email linked with paypal account | [optional] 
**telephone_number** | **str** | mobile number linked to paypal account | [optional] 
**paypal_id** | **str** | id of the paypal account | [optional] 

## Example

```python
from hyperswitch.models.paypal_additional_data import PaypalAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of PaypalAdditionalData from a JSON string
paypal_additional_data_instance = PaypalAdditionalData.from_json(json)
# print the JSON string representation of the object
print(PaypalAdditionalData.to_json())

# convert the object into a dict
paypal_additional_data_dict = paypal_additional_data_instance.to_dict()
# create an instance of PaypalAdditionalData from a dict
paypal_additional_data_from_dict = PaypalAdditionalData.from_dict(paypal_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


