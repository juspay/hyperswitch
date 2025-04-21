# VoucherResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**boleto** | [**BoletoVoucherData**](BoletoVoucherData.md) |  | 
**alfamart** | [**AlfamartVoucherData**](AlfamartVoucherData.md) |  | 
**indomaret** | [**IndomaretVoucherData**](IndomaretVoucherData.md) |  | 
**seven_eleven** | [**JCSVoucherData**](JCSVoucherData.md) |  | 
**lawson** | [**JCSVoucherData**](JCSVoucherData.md) |  | 
**mini_stop** | [**JCSVoucherData**](JCSVoucherData.md) |  | 
**family_mart** | [**JCSVoucherData**](JCSVoucherData.md) |  | 
**seicomart** | [**JCSVoucherData**](JCSVoucherData.md) |  | 
**pay_easy** | [**JCSVoucherData**](JCSVoucherData.md) |  | 

## Example

```python
from hyperswitch.models.voucher_response import VoucherResponse

# TODO update the JSON string below
json = "{}"
# create an instance of VoucherResponse from a JSON string
voucher_response_instance = VoucherResponse.from_json(json)
# print the JSON string representation of the object
print(VoucherResponse.to_json())

# convert the object into a dict
voucher_response_dict = voucher_response_instance.to_dict()
# create an instance of VoucherResponse from a dict
voucher_response_from_dict = VoucherResponse.from_dict(voucher_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


