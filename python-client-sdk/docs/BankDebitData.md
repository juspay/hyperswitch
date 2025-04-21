# BankDebitData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach_bank_debit** | [**BankDebitDataOneOfAchBankDebit**](BankDebitDataOneOfAchBankDebit.md) |  | 
**sepa_bank_debit** | [**BankDebitDataOneOf1SepaBankDebit**](BankDebitDataOneOf1SepaBankDebit.md) |  | 
**becs_bank_debit** | [**BankDebitDataOneOf2BecsBankDebit**](BankDebitDataOneOf2BecsBankDebit.md) |  | 
**bacs_bank_debit** | [**BankDebitDataOneOf3BacsBankDebit**](BankDebitDataOneOf3BacsBankDebit.md) |  | 

## Example

```python
from hyperswitch.models.bank_debit_data import BankDebitData

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitData from a JSON string
bank_debit_data_instance = BankDebitData.from_json(json)
# print the JSON string representation of the object
print(BankDebitData.to_json())

# convert the object into a dict
bank_debit_data_dict = bank_debit_data_instance.to_dict()
# create an instance of BankDebitData from a dict
bank_debit_data_from_dict = BankDebitData.from_dict(bank_debit_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


