# IndomaretVoucherData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**first_name** | **str** | The billing first name for Alfamart | [optional] 
**last_name** | **str** | The billing second name for Alfamart | [optional] 
**email** | **str** | The Email ID for Alfamart | [optional] 

## Example

```python
from hyperswitch.models.indomaret_voucher_data import IndomaretVoucherData

# TODO update the JSON string below
json = "{}"
# create an instance of IndomaretVoucherData from a JSON string
indomaret_voucher_data_instance = IndomaretVoucherData.from_json(json)
# print the JSON string representation of the object
print(IndomaretVoucherData.to_json())

# convert the object into a dict
indomaret_voucher_data_dict = indomaret_voucher_data_instance.to_dict()
# create an instance of IndomaretVoucherData from a dict
indomaret_voucher_data_from_dict = IndomaretVoucherData.from_dict(indomaret_voucher_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


