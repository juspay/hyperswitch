# BankRedirectDetailsOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bancontact_card** | [**BancontactBankRedirectAdditionalData**](BancontactBankRedirectAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_details_one_of import BankRedirectDetailsOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDetailsOneOf from a JSON string
bank_redirect_details_one_of_instance = BankRedirectDetailsOneOf.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDetailsOneOf.to_json())

# convert the object into a dict
bank_redirect_details_one_of_dict = bank_redirect_details_one_of_instance.to_dict()
# create an instance of BankRedirectDetailsOneOf from a dict
bank_redirect_details_one_of_from_dict = BankRedirectDetailsOneOf.from_dict(bank_redirect_details_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


