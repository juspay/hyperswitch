# BankDebitDataOneOf2BecsBankDebit


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankDebitBilling**](BankDebitBilling.md) |  | [optional] 
**account_number** | **str** | Account number for Becs payment method | 
**bsb_number** | **str** | Bank-State-Branch (bsb) number | 
**bank_account_holder_name** | **str** | Owner name for bank debit | [optional] 

## Example

```python
from hyperswitch.models.bank_debit_data_one_of2_becs_bank_debit import BankDebitDataOneOf2BecsBankDebit

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitDataOneOf2BecsBankDebit from a JSON string
bank_debit_data_one_of2_becs_bank_debit_instance = BankDebitDataOneOf2BecsBankDebit.from_json(json)
# print the JSON string representation of the object
print(BankDebitDataOneOf2BecsBankDebit.to_json())

# convert the object into a dict
bank_debit_data_one_of2_becs_bank_debit_dict = bank_debit_data_one_of2_becs_bank_debit_instance.to_dict()
# create an instance of BankDebitDataOneOf2BecsBankDebit from a dict
bank_debit_data_one_of2_becs_bank_debit_from_dict = BankDebitDataOneOf2BecsBankDebit.from_dict(bank_debit_data_one_of2_becs_bank_debit_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


