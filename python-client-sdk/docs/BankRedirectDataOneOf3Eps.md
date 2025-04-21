# BankRedirectDataOneOf3Eps


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankRedirectBilling**](BankRedirectBilling.md) |  | [optional] 
**bank_name** | [**BankNames**](BankNames.md) |  | 
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_data_one_of3_eps import BankRedirectDataOneOf3Eps

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDataOneOf3Eps from a JSON string
bank_redirect_data_one_of3_eps_instance = BankRedirectDataOneOf3Eps.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDataOneOf3Eps.to_json())

# convert the object into a dict
bank_redirect_data_one_of3_eps_dict = bank_redirect_data_one_of3_eps_instance.to_dict()
# create an instance of BankRedirectDataOneOf3Eps from a dict
bank_redirect_data_one_of3_eps_from_dict = BankRedirectDataOneOf3Eps.from_dict(bank_redirect_data_one_of3_eps_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


