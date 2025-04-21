# BankDebitBilling


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** | The billing name for bank debits | [optional] 
**email** | **str** | The billing email for bank debits | [optional] 
**address** | [**AddressDetails**](AddressDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.bank_debit_billing import BankDebitBilling

# TODO update the JSON string below
json = "{}"
# create an instance of BankDebitBilling from a JSON string
bank_debit_billing_instance = BankDebitBilling.from_json(json)
# print the JSON string representation of the object
print(BankDebitBilling.to_json())

# convert the object into a dict
bank_debit_billing_dict = bank_debit_billing_instance.to_dict()
# create an instance of BankDebitBilling from a dict
bank_debit_billing_from_dict = BankDebitBilling.from_dict(bank_debit_billing_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


