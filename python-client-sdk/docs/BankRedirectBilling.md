# BankRedirectBilling


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_name** | **str** | The name for which billing is issued | 
**email** | **str** | The billing email for bank redirect | 

## Example

```python
from hyperswitch.models.bank_redirect_billing import BankRedirectBilling

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectBilling from a JSON string
bank_redirect_billing_instance = BankRedirectBilling.from_json(json)
# print the JSON string representation of the object
print(BankRedirectBilling.to_json())

# convert the object into a dict
bank_redirect_billing_dict = bank_redirect_billing_instance.to_dict()
# create an instance of BankRedirectBilling from a dict
bank_redirect_billing_from_dict = BankRedirectBilling.from_dict(bank_redirect_billing_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


