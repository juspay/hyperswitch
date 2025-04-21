# BankTransferResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach** | **object** |  | 
**sepa** | **object** |  | 
**bacs** | **object** |  | 
**multibanco** | **object** |  | 
**permata** | **object** |  | 
**bca** | **object** |  | 
**bni_va** | **object** |  | 
**bri_va** | **object** |  | 
**cimb_va** | **object** |  | 
**danamon_va** | **object** |  | 
**mandiri_va** | **object** |  | 
**pix** | [**PixBankTransferAdditionalData**](PixBankTransferAdditionalData.md) |  | 
**pse** | **object** |  | 
**local_bank_transfer** | [**LocalBankTransferAdditionalData**](LocalBankTransferAdditionalData.md) |  | 
**instant_bank_transfer** | **object** |  | 

## Example

```python
from hyperswitch.models.bank_transfer_response import BankTransferResponse

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferResponse from a JSON string
bank_transfer_response_instance = BankTransferResponse.from_json(json)
# print the JSON string representation of the object
print(BankTransferResponse.to_json())

# convert the object into a dict
bank_transfer_response_dict = bank_transfer_response_instance.to_dict()
# create an instance of BankTransferResponse from a dict
bank_transfer_response_from_dict = BankTransferResponse.from_dict(bank_transfer_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


