# BankTransferInstructions


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**doku_bank_transfer_instructions** | [**DokuBankTransferInstructions**](DokuBankTransferInstructions.md) |  | 
**ach_credit_transfer** | [**AchTransfer**](AchTransfer.md) |  | 
**sepa_bank_instructions** | [**SepaBankTransferInstructions**](SepaBankTransferInstructions.md) |  | 
**bacs_bank_instructions** | [**BacsBankTransferInstructions**](BacsBankTransferInstructions.md) |  | 
**multibanco** | [**MultibancoTransferInstructions**](MultibancoTransferInstructions.md) |  | 

## Example

```python
from hyperswitch.models.bank_transfer_instructions import BankTransferInstructions

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferInstructions from a JSON string
bank_transfer_instructions_instance = BankTransferInstructions.from_json(json)
# print the JSON string representation of the object
print(BankTransferInstructions.to_json())

# convert the object into a dict
bank_transfer_instructions_dict = bank_transfer_instructions_instance.to_dict()
# create an instance of BankTransferInstructions from a dict
bank_transfer_instructions_from_dict = BankTransferInstructions.from_dict(bank_transfer_instructions_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


