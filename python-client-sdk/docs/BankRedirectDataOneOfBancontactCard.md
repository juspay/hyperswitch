# BankRedirectDataOneOfBancontactCard


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card_number** | **str** | The card number | 
**card_exp_month** | **str** | The card&#39;s expiry month | 
**card_exp_year** | **str** | The card&#39;s expiry year | 
**card_holder_name** | **str** | The card holder&#39;s name | 
**billing_details** | [**BankRedirectBilling**](BankRedirectBilling.md) |  | [optional] 

## Example

```python
from hyperswitch.models.bank_redirect_data_one_of_bancontact_card import BankRedirectDataOneOfBancontactCard

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectDataOneOfBancontactCard from a JSON string
bank_redirect_data_one_of_bancontact_card_instance = BankRedirectDataOneOfBancontactCard.from_json(json)
# print the JSON string representation of the object
print(BankRedirectDataOneOfBancontactCard.to_json())

# convert the object into a dict
bank_redirect_data_one_of_bancontact_card_dict = bank_redirect_data_one_of_bancontact_card_instance.to_dict()
# create an instance of BankRedirectDataOneOfBancontactCard from a dict
bank_redirect_data_one_of_bancontact_card_from_dict = BankRedirectDataOneOfBancontactCard.from_dict(bank_redirect_data_one_of_bancontact_card_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


