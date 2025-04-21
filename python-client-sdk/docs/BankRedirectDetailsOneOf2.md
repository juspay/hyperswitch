# BankRedirectDetailsOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**giropay** | [**GiropayBankRedirectAdditionalData**](GiropayBankRedirectAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_details_one_of2 import BankRedirectDetailsOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDetailsOneOf2 from a JSON string
bank_redirect_details_one_of2_instance = BankRedirectDetailsOneOf2.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDetailsOneOf2.to_json())

# convert the object into a dict
bank_redirect_details_one_of2_dict = bank_redirect_details_one_of2_instance.to_dict()
# create an instance of BankRedirectDetailsOneOf2 from a dict
bank_redirect_details_one_of2_from_dict = BankRedirectDetailsOneOf2.from_dict(bank_redirect_details_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


