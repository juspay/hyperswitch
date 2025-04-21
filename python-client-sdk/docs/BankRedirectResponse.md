# BankRedirectResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bancontact_card** | [**BancontactBankRedirectAdditionalData**](BancontactBankRedirectAdditionalData.md) |  | 
**blik** | [**BlikBankRedirectAdditionalData**](BlikBankRedirectAdditionalData.md) |  | 
**giropay** | [**GiropayBankRedirectAdditionalData**](GiropayBankRedirectAdditionalData.md) |  | 
**bank_name** | [**BankNames**](BankNames.md) |  | [optional] 

## Example

```python
from hyperswitch.models.bank_redirect_response import BankRedirectResponse

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectResponse from a JSON string
bank_redirect_response_instance = BankRedirectResponse.from_json(json)
# print the JSON string representation of the object
print(BankRedirectResponse.to_json())

# convert the object into a dict
bank_redirect_response_dict = bank_redirect_response_instance.to_dict()
# create an instance of BankRedirectResponse from a dict
bank_redirect_response_from_dict = BankRedirectResponse.from_dict(bank_redirect_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


