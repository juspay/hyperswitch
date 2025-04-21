# BankDebit


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_debit** | [**BankDebitData**](BankDebitData.md) |  | 

## Example

```python
from hyperswitch.models.bank_debit import BankDebit

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebit from a JSON string
bank_debit_instance = BankDebit.from_json(json)
# print the JSON string representation of the object
print(BankDebit.to_json())

# convert the object into a dict
bank_debit_dict = bank_debit_instance.to_dict()
# create an instance of BankDebit from a dict
bank_debit_from_dict = BankDebit.from_dict(bank_debit_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


