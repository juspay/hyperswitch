# BankTransferNextStepsData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**doku_bank_transfer_instructions** | [**DokuBankTransferInstructions**](DokuBankTransferInstructions.md) |  | 
**ach_credit_transfer** | [**AchTransfer**](AchTransfer.md) |  | 
**sepa_bank_instructions** | [**SepaBankTransferInstructions**](SepaBankTransferInstructions.md) |  | 
**bacs_bank_instructions** | [**BacsBankTransferInstructions**](BacsBankTransferInstructions.md) |  | 
**multibanco** | [**MultibancoTransferInstructions**](MultibancoTransferInstructions.md) |  | 
**receiver** | [**ReceiverDetails**](ReceiverDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.bank_transfer_next_steps_data import BankTransferNextStepsData

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferNextStepsData from a JSON string
bank_transfer_next_steps_data_instance = BankTransferNextStepsData.from_json(json)
# print the JSON string representation of the object
print(BankTransferNextStepsData.to_json())

# convert the object into a dict
bank_transfer_next_steps_data_dict = bank_transfer_next_steps_data_instance.to_dict()
# create an instance of BankTransferNextStepsData from a dict
bank_transfer_next_steps_data_from_dict = BankTransferNextStepsData.from_dict(bank_transfer_next_steps_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


