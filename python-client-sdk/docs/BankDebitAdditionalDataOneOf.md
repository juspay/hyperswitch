# BankDebitAdditionalDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**ach** | [**AchBankDebitAdditionalData**](AchBankDebitAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.bank_debit_additional_data_one_of import BankDebitAdditionalDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitAdditionalDataOneOf from a JSON string
bank_debit_additional_data_one_of_instance = BankDebitAdditionalDataOneOf.from_json(json)
# print the JSON string representation of the object
print(BankDebitAdditionalDataOneOf.to_json())

# convert the object into a dict
bank_debit_additional_data_one_of_dict = bank_debit_additional_data_one_of_instance.to_dict()
# create an instance of BankDebitAdditionalDataOneOf from a dict
bank_debit_additional_data_one_of_from_dict = BankDebitAdditionalDataOneOf.from_dict(bank_debit_additional_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


