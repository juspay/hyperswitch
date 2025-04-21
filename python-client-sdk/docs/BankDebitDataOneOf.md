# BankDebitDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach_bank_debit** | [**BankDebitDataOneOfAchBankDebit**](BankDebitDataOneOfAchBankDebit.md) |  | 

## Example

```python
from hyperswitch.models.bank_debit_data_one_of import BankDebitDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitDataOneOf from a JSON string
bank_debit_data_one_of_instance = BankDebitDataOneOf.from_json(json)
# print the JSON string representation of the object
print(BankDebitDataOneOf.to_json())

# convert the object into a dict
bank_debit_data_one_of_dict = bank_debit_data_one_of_instance.to_dict()
# create an instance of BankDebitDataOneOf from a dict
bank_debit_data_one_of_from_dict = BankDebitDataOneOf.from_dict(bank_debit_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


