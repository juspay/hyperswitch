# BankRedirectDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bancontact_card** | [**BankRedirectDataOneOfBancontactCard**](BankRedirectDataOneOfBancontactCard.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_data_one_of import BankRedirectDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDataOneOf from a JSON string
bank_redirect_data_one_of_instance = BankRedirectDataOneOf.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDataOneOf.to_json())

# convert the object into a dict
bank_redirect_data_one_of_dict = bank_redirect_data_one_of_instance.to_dict()
# create an instance of BankRedirectDataOneOf from a dict
bank_redirect_data_one_of_from_dict = BankRedirectDataOneOf.from_dict(bank_redirect_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


