# BankRedirectDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bancontact_card** | [**BancontactBankRedirectAdditionalData**](BancontactBankRedirectAdditionalData.md) |  | 
**blik** | [**BlikBankRedirectAdditionalData**](BlikBankRedirectAdditionalData.md) |  | 
**giropay** | [**GiropayBankRedirectAdditionalData**](GiropayBankRedirectAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_details import BankRedirectDetails

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDetails from a JSON string
bank_redirect_details_instance = BankRedirectDetails.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDetails.to_json())

# convert the object into a dict
bank_redirect_details_dict = bank_redirect_details_instance.to_dict()
# create an instance of BankRedirectDetails from a dict
bank_redirect_details_from_dict = BankRedirectDetails.from_dict(bank_redirect_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


