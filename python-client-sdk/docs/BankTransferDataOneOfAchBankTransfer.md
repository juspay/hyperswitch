# BankTransferDataOneOfAchBankTransfer


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**AchBillingDetails**](AchBillingDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.bank_transfer_data_one_of_ach_bank_transfer import BankTransferDataOneOfAchBankTransfer

# TODO update the JSON string below
json = "{}"
# create an instance of BankTransferDataOneOfAchBankTransfer from a JSON string
bank_transfer_data_one_of_ach_bank_transfer_instance = BankTransferDataOneOfAchBankTransfer.from_json(json)
# print the JSON string representation of the object
print(BankTransferDataOneOfAchBankTransfer.to_json())

# convert the object into a dict
bank_transfer_data_one_of_ach_bank_transfer_dict = bank_transfer_data_one_of_ach_bank_transfer_instance.to_dict()
# create an instance of BankTransferDataOneOfAchBankTransfer from a dict
bank_transfer_data_one_of_ach_bank_transfer_from_dict = BankTransferDataOneOfAchBankTransfer.from_dict(bank_transfer_data_one_of_ach_bank_transfer_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


