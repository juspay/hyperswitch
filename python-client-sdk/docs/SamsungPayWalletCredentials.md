# SamsungPayWalletCredentials


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**method** | **str** | Specifies authentication method used | [optional] 
**recurring_payment** | **bool** | Value if credential is enabled for recurring payment | [optional] 
**card_brand** | [**SamsungPayCardBrand**](SamsungPayCardBrand.md) |  | 
**card_last4digits** | **str** | Last 4 digits of the card number | 
**var_3_d_s** | [**SamsungPayTokenData**](SamsungPayTokenData.md) |  | 
**payment_card_brand** | [**SamsungPayCardBrand**](SamsungPayCardBrand.md) |  | 
**payment_currency_type** | **str** | Currency type of the payment | 
**payment_last4_dpan** | **str** | Last 4 digits of the device specific card number | [optional] 
**payment_last4_fpan** | **str** | Last 4 digits of the card number | 
**merchant_ref** | **str** | Merchant reference id that was passed in the session call request | [optional] 

## Example

```python
from hyperswitch.models.samsung_pay_wallet_credentials import SamsungPayWalletCredentials

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPayWalletCredentials from a JSON string
samsung_pay_wallet_credentials_instance = SamsungPayWalletCredentials.from_json(json)
# print the JSON string representation of the object
print(SamsungPayWalletCredentials.to_json())

# convert the object into a dict
samsung_pay_wallet_credentials_dict = samsung_pay_wallet_credentials_instance.to_dict()
# create an instance of SamsungPayWalletCredentials from a dict
samsung_pay_wallet_credentials_from_dict = SamsungPayWalletCredentials.from_dict(samsung_pay_wallet_credentials_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


