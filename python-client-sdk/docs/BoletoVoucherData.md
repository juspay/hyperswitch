# BoletoVoucherData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**social_security_number** | **str** | The shopper&#39;s social security number | [optional] 

## Example

```python
from hyperswitch.models.boleto_voucher_data import BoletoVoucherData

# TODO update the JSON string below
json = "{}"
# create an instance of BoletoVoucherData from a JSON string
boleto_voucher_data_instance = BoletoVoucherData.from_json(json)
# print the JSON string representation of the object
print(BoletoVoucherData.to_json())

# convert the object into a dict
boleto_voucher_data_dict = boleto_voucher_data_instance.to_dict()
# create an instance of BoletoVoucherData from a dict
boleto_voucher_data_from_dict = BoletoVoucherData.from_dict(boleto_voucher_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


