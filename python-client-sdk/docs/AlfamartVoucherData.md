# AlfamartVoucherData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**first_name** | **str** | The billing first name for Alfamart | [optional] 
**last_name** | **str** | The billing second name for Alfamart | [optional] 
**email** | **str** | The Email ID for Alfamart | [optional] 

## Example

```python
from hyperswitch.models.alfamart_voucher_data import AlfamartVoucherData

# TODO update the JSON string below
json = "{}"
# create an instance of AlfamartVoucherData from a JSON string
alfamart_voucher_data_instance = AlfamartVoucherData.from_json(json)
# print the JSON string representation of the object
print(AlfamartVoucherData.to_json())

# convert the object into a dict
alfamart_voucher_data_dict = alfamart_voucher_data_instance.to_dict()
# create an instance of AlfamartVoucherData from a dict
alfamart_voucher_data_from_dict = AlfamartVoucherData.from_dict(alfamart_voucher_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


