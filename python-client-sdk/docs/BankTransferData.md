# BankTransferData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach_bank_transfer** | [**BankTransferDataOneOfAchBankTransfer**](BankTransferDataOneOfAchBankTransfer.md) |  | 
**sepa_bank_transfer** | [**BankTransferDataOneOf1SepaBankTransfer**](BankTransferDataOneOf1SepaBankTransfer.md) |  | 
**bacs_bank_transfer** | [**BankTransferDataOneOf2BacsBankTransfer**](BankTransferDataOneOf2BacsBankTransfer.md) |  | 
**multibanco_bank_transfer** | [**BankTransferDataOneOf3MultibancoBankTransfer**](BankTransferDataOneOf3MultibancoBankTransfer.md) |  | 
**permata_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**bca_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**bni_va_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**bri_va_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**cimb_va_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**danamon_va_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**mandiri_va_bank_transfer** | [**BankTransferDataOneOf4PermataBankTransfer**](BankTransferDataOneOf4PermataBankTransfer.md) |  | 
**pix** | [**BankTransferDataOneOf11Pix**](BankTransferDataOneOf11Pix.md) |  | 
**pse** | **object** |  | 
**local_bank_transfer** | [**BankTransferDataOneOf12LocalBankTransfer**](BankTransferDataOneOf12LocalBankTransfer.md) |  | 
**instant_bank_transfer** | **object** |  | 

## Example

```python
from hyperswitch.models.bank_transfer_data import BankTransferData

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferData from a JSON string
bank_transfer_data_instance = BankTransferData.from_json(json)
# print the JSON string representation of the object
print(BankTransferData.to_json())

# convert the object into a dict
bank_transfer_data_dict = bank_transfer_data_instance.to_dict()
# create an instance of BankTransferData from a dict
bank_transfer_data_from_dict = BankTransferData.from_dict(bank_transfer_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


