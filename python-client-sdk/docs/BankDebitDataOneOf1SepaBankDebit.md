# BankDebitDataOneOf1SepaBankDebit


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankDebitBilling**](BankDebitBilling.md) |  | [optional] 
**iban** | **str** | International bank account number (iban) for SEPA | 
**bank_account_holder_name** | **str** | Owner name for bank debit | 

## Example

```python
from hyperswitch.models.bank_debit_data_one_of1_sepa_bank_debit import BankDebitDataOneOf1SepaBankDebit

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitDataOneOf1SepaBankDebit from a JSON string
bank_debit_data_one_of1_sepa_bank_debit_instance = BankDebitDataOneOf1SepaBankDebit.from_json(json)
# print the JSON string representation of the object
print(BankDebitDataOneOf1SepaBankDebit.to_json())

# convert the object into a dict
bank_debit_data_one_of1_sepa_bank_debit_dict = bank_debit_data_one_of1_sepa_bank_debit_instance.to_dict()
# create an instance of BankDebitDataOneOf1SepaBankDebit from a dict
bank_debit_data_one_of1_sepa_bank_debit_from_dict = BankDebitDataOneOf1SepaBankDebit.from_dict(bank_debit_data_one_of1_sepa_bank_debit_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


