# BankDebitDataOneOfAchBankDebit

Payment Method data for Ach bank debit

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankDebitBilling**](BankDebitBilling.md) |  | [optional] 
**account_number** | **str** | Account number for ach bank debit payment | 
**routing_number** | **str** | Routing number for ach bank debit payment | 
**card_holder_name** | **str** |  | 
**bank_account_holder_name** | **str** |  | 
**bank_name** | **str** |  | 
**bank_type** | **str** |  | 
**bank_holder_type** | **str** |  | 

## Example

```python
from hyperswitch.models.bank_debit_data_one_of_ach_bank_debit import BankDebitDataOneOfAchBankDebit

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitDataOneOfAchBankDebit from a JSON string
bank_debit_data_one_of_ach_bank_debit_instance = BankDebitDataOneOfAchBankDebit.from_json(json)
# print the JSON string representation of the object
print(BankDebitDataOneOfAchBankDebit.to_json())

# convert the object into a dict
bank_debit_data_one_of_ach_bank_debit_dict = bank_debit_data_one_of_ach_bank_debit_instance.to_dict()
# create an instance of BankDebitDataOneOfAchBankDebit from a dict
bank_debit_data_one_of_ach_bank_debit_from_dict = BankDebitDataOneOfAchBankDebit.from_dict(bank_debit_data_one_of_ach_bank_debit_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


