# BankDebitDataOneOf3BacsBankDebit


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankDebitBilling**](BankDebitBilling.md) |  | [optional] 
**account_number** | **str** | Account number for Bacs payment method | 
**sort_code** | **str** | Sort code for Bacs payment method | 
**bank_account_holder_name** | **str** | holder name for bank debit | 

## Example

```python
from hyperswitch.models.bank_debit_data_one_of3_bacs_bank_debit import BankDebitDataOneOf3BacsBankDebit

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitDataOneOf3BacsBankDebit from a JSON string
bank_debit_data_one_of3_bacs_bank_debit_instance = BankDebitDataOneOf3BacsBankDebit.from_json(json)
# print the JSON string representation of the object
print(BankDebitDataOneOf3BacsBankDebit.to_json())

# convert the object into a dict
bank_debit_data_one_of3_bacs_bank_debit_dict = bank_debit_data_one_of3_bacs_bank_debit_instance.to_dict()
# create an instance of BankDebitDataOneOf3BacsBankDebit from a dict
bank_debit_data_one_of3_bacs_bank_debit_from_dict = BankDebitDataOneOf3BacsBankDebit.from_dict(bank_debit_data_one_of3_bacs_bank_debit_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


