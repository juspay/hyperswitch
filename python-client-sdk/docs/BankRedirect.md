# BankRedirect


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bank_redirect** | [**BankRedirectData**](BankRedirectData.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect import BankRedirect

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirect from a JSON string
bank_redirect_instance = BankRedirect.from_json(json)
# print the JSON string representation of the object
print(BankRedirect.to_json())

# convert the object into a dict
bank_redirect_dict = bank_redirect_instance.to_dict()
# create an instance of BankRedirect from a dict
bank_redirect_from_dict = BankRedirect.from_dict(bank_redirect_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


