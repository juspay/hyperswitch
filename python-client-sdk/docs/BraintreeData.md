# BraintreeData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_account_id** | **str** | Information about the merchant_account_id that merchant wants to specify at connector level. | 
**merchant_config_currency** | **str** | Information about the merchant_config_currency that merchant wants to specify at connector level. | 

## Example

```python
from hyperswitch.models.braintree_data import BraintreeData

# TODO update the JSON string below
json = "{}"
# create an instance of BraintreeData from a JSON string
braintree_data_instance = BraintreeData.from_json(json)
# print the JSON string representation of the object
print(BraintreeData.to_json())

# convert the object into a dict
braintree_data_dict = braintree_data_instance.to_dict()
# create an instance of BraintreeData from a dict
braintree_data_from_dict = BraintreeData.from_dict(braintree_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


