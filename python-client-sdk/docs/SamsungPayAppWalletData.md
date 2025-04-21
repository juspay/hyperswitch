# SamsungPayAppWalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**var_3_d_s** | [**SamsungPayTokenData**](SamsungPayTokenData.md) |  | 
**payment_card_brand** | [**SamsungPayCardBrand**](SamsungPayCardBrand.md) |  | 
**payment_currency_type** | **str** | Currency type of the payment | 
**payment_last4_dpan** | **str** | Last 4 digits of the device specific card number | [optional] 
**payment_last4_fpan** | **str** | Last 4 digits of the card number | 
**merchant_ref** | **str** | Merchant reference id that was passed in the session call request | [optional] 
**method** | **str** | Specifies authentication method used | [optional] 
**recurring_payment** | **bool** | Value if credential is enabled for recurring payment | [optional] 

## Example

```python
from hyperswitch.models.samsung_pay_app_wallet_data import SamsungPayAppWalletData

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPayAppWalletData from a JSON string
samsung_pay_app_wallet_data_instance = SamsungPayAppWalletData.from_json(json)
# print the JSON string representation of the object
print(SamsungPayAppWalletData.to_json())

# convert the object into a dict
samsung_pay_app_wallet_data_dict = samsung_pay_app_wallet_data_instance.to_dict()
# create an instance of SamsungPayAppWalletData from a dict
samsung_pay_app_wallet_data_from_dict = SamsungPayAppWalletData.from_dict(samsung_pay_app_wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


