# BankTransferDataOneOf11Pix


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**pix_key** | **str** | Unique key for pix transfer | [optional] 
**cpf** | **str** | CPF is a Brazilian tax identification number | [optional] 
**cnpj** | **str** | CNPJ is a Brazilian company tax identification number | [optional] 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of11_pix import BankTransferDataOneOf11Pix

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOf11Pix from a JSON string
bank_transfer_data_one_of11_pix_instance = BankTransferDataOneOf11Pix.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOf11Pix.to_json())

# convert the object into a dict
bank_transfer_data_one_of11_pix_dict = bank_transfer_data_one_of11_pix_instance.to_dict()
# create an instance of BankTransferDataOneOf11Pix from a dict
bank_transfer_data_one_of11_pix_from_dict = BankTransferDataOneOf11Pix.from_dict(bank_transfer_data_one_of11_pix_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


