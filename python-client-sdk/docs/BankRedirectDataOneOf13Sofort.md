# BankRedirectDataOneOf13Sofort


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**billing_details** | [**BankRedirectBilling**](BankRedirectBilling.md) |  | [optional] 
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | 
**preferred_language** | **str** | The preferred language | [optional] 

## Example

```python
from hyperswitch.models.bank_redirect_data_one_of13_sofort import BankRedirectDataOneOf13Sofort

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDataOneOf13Sofort from a JSON string
bank_redirect_data_one_of13_sofort_instance = BankRedirectDataOneOf13Sofort.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDataOneOf13Sofort.to_json())

# convert the object into a dict
bank_redirect_data_one_of13_sofort_dict = bank_redirect_data_one_of13_sofort_instance.to_dict()
# create an instance of BankRedirectDataOneOf13Sofort from a dict
bank_redirect_data_one_of13_sofort_from_dict = BankRedirectDataOneOf13Sofort.from_dict(bank_redirect_data_one_of13_sofort_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


