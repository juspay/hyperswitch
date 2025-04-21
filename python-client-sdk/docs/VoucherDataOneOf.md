# VoucherDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**boleto** | [**BoletoVoucherData**](BoletoVoucherData.md) |  | 

## Example

```python
from hyperswitch.models.voucher_data_one_of import VoucherDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of VoucherDataOneOf from a JSON string
voucher_data_one_of_instance = VoucherDataOneOf.from_json(json)
# print the JSON string representation of the object
print(VoucherDataOneOf.to_json())

# convert the object into a dict
voucher_data_one_of_dict = voucher_data_one_of_instance.to_dict()
# create an instance of VoucherDataOneOf from a dict
voucher_data_one_of_from_dict = VoucherDataOneOf.from_dict(voucher_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


