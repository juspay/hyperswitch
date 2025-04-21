# BancontactBankRedirectAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**last4** | **str** | Last 4 digits of the card number | [optional] 
**card_exp_month** | **str** | The card&#39;s expiry month | [optional] 
**card_exp_year** | **str** | The card&#39;s expiry year | [optional] 
**card_holder_name** | **str** | The card holder&#39;s name | [optional] 

## Example

```python
from hyperswitch.models.bancontact_bank_redirect_additional_data import BancontactBankRedirectAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of BancontactBankRedirectAdditionalData from a JSON string
bancontact_bank_redirect_additional_data_instance = BancontactBankRedirectAdditionalData.from_json(json)
# print the JSON string representation of the object
print(BancontactBankRedirectAdditionalData.to_json())

# convert the object into a dict
bancontact_bank_redirect_additional_data_dict = bancontact_bank_redirect_additional_data_instance.to_dict()
# create an instance of BancontactBankRedirectAdditionalData from a dict
bancontact_bank_redirect_additional_data_from_dict = BancontactBankRedirectAdditionalData.from_dict(bancontact_bank_redirect_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


