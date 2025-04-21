# BankRedirectData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**bancontact_card** | [**BankRedirectDataOneOfBancontactCard**](BankRedirectDataOneOfBancontactCard.md) |  | 
**bizum** | **object** |  | 
**blik** | [**BankRedirectDataOneOf2Blik**](BankRedirectDataOneOf2Blik.md) |  | 
**eps** | [**BankRedirectDataOneOf3Eps**](BankRedirectDataOneOf3Eps.md) |  | 
**giropay** | [**BankRedirectDataOneOf4Giropay**](BankRedirectDataOneOf4Giropay.md) |  | 
**ideal** | [**BankRedirectDataOneOf3Eps**](BankRedirectDataOneOf3Eps.md) |  | 
**interac** | [**BankRedirectDataOneOf6Interac**](BankRedirectDataOneOf6Interac.md) |  | 
**online_banking_czech_republic** | [**BankRedirectDataOneOf7OnlineBankingCzechRepublic**](BankRedirectDataOneOf7OnlineBankingCzechRepublic.md) |  | 
**online_banking_finland** | [**BankRedirectDataOneOf8OnlineBankingFinland**](BankRedirectDataOneOf8OnlineBankingFinland.md) |  | 
**online_banking_poland** | [**BankRedirectDataOneOf7OnlineBankingCzechRepublic**](BankRedirectDataOneOf7OnlineBankingCzechRepublic.md) |  | 
**online_banking_slovakia** | [**BankRedirectDataOneOf7OnlineBankingCzechRepublic**](BankRedirectDataOneOf7OnlineBankingCzechRepublic.md) |  | 
**open_banking_uk** | [**BankRedirectDataOneOf11OpenBankingUk**](BankRedirectDataOneOf11OpenBankingUk.md) |  | 
**przelewy24** | [**BankRedirectDataOneOf12Przelewy24**](BankRedirectDataOneOf12Przelewy24.md) |  | 
**sofort** | [**BankRedirectDataOneOf13Sofort**](BankRedirectDataOneOf13Sofort.md) |  | 
**trustly** | [**BankRedirectDataOneOf14Trustly**](BankRedirectDataOneOf14Trustly.md) |  | 
**online_banking_fpx** | [**BankRedirectDataOneOf7OnlineBankingCzechRepublic**](BankRedirectDataOneOf7OnlineBankingCzechRepublic.md) |  | 
**online_banking_thailand** | [**BankRedirectDataOneOf7OnlineBankingCzechRepublic**](BankRedirectDataOneOf7OnlineBankingCzechRepublic.md) |  | 
**local_bank_redirect** | **object** |  | 
**eft** | [**BankRedirectDataOneOf18Eft**](BankRedirectDataOneOf18Eft.md) |  | 

## Example

```python
from hyperswitch.models.bank_redirect_data import BankRedirectData

# TODO update the JSON string below
json = "{}"
# create an instance of BankRedirectData from a JSON string
bank_redirect_data_instance = BankRedirectData.from_json(json)
# print the JSON string representation of the object
print(BankRedirectData.to_json())

# convert the object into a dict
bank_redirect_data_dict = bank_redirect_data_instance.to_dict()
# create an instance of BankRedirectData from a dict
bank_redirect_data_from_dict = BankRedirectData.from_dict(bank_redirect_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


