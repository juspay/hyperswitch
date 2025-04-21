# JCSVoucherData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**first_name** | **str** | The billing first name for Japanese convenience stores | [optional] 
**last_name** | **str** | The billing second name Japanese convenience stores | [optional] 
**email** | **str** | The Email ID for Japanese convenience stores | [optional] 
**phone_number** | **str** | The telephone number for Japanese convenience stores | [optional] 

## Example

```python
from hyperswitch.models.jcs_voucher_data import JCSVoucherData

# TODO update the JSON string below
json = "{}"
# create an instance of JCSVoucherData from a JSON string
jcs_voucher_data_instance = JCSVoucherData.from_json(json)
# print the JSON string representation of the object
print(JCSVoucherData.to_json())

# convert the object into a dict
jcs_voucher_data_dict = jcs_voucher_data_instance.to_dict()
# create an instance of JCSVoucherData from a dict
jcs_voucher_data_from_dict = JCSVoucherData.from_dict(jcs_voucher_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


