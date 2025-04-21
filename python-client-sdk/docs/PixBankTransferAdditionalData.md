# PixBankTransferAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**pix_key** | **str** | Partially masked unique key for pix transfer | [optional] 
**cpf** | **str** | Partially masked CPF - CPF is a Brazilian tax identification number | [optional] 
**cnpj** | **str** | Partially masked CNPJ - CNPJ is a Brazilian company tax identification number | [optional] 

## Example

```python
from hyperswitch.models.pix_bank_transfer_additional_data import PixBankTransferAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of PixBankTransferAdditionalData from a JSON string
pix_bank_transfer_additional_data_instance = PixBankTransferAdditionalData.from_json(json)
# print the JSON string representation of the object
print(PixBankTransferAdditionalData.to_json())

# convert the object into a dict
pix_bank_transfer_additional_data_dict = pix_bank_transfer_additional_data_instance.to_dict()
# create an instance of PixBankTransferAdditionalData from a dict
pix_bank_transfer_additional_data_from_dict = PixBankTransferAdditionalData.from_dict(pix_bank_transfer_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


