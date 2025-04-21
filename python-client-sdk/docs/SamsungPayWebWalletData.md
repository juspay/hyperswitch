# SamsungPayWebWalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**method** | **str** | Specifies authentication method used | [optional] 
**recurring_payment** | **bool** | Value if credential is enabled for recurring payment | [optional] 
**card_brand** | [**SamsungPayCardBrand**](SamsungPayCardBrand.md) |  | 
**card_last4digits** | **str** | Last 4 digits of the card number | 
**var_3_d_s** | [**SamsungPayTokenData**](SamsungPayTokenData.md) |  | 

## Example

```python
from hyperswitch.models.samsung_pay_web_wallet_data import SamsungPayWebWalletData

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPayWebWalletData from a JSON string
samsung_pay_web_wallet_data_instance = SamsungPayWebWalletData.from_json(json)
# print the JSON string representation of the object
print(SamsungPayWebWalletData.to_json())

# convert the object into a dict
samsung_pay_web_wallet_data_dict = samsung_pay_web_wallet_data_instance.to_dict()
# create an instance of SamsungPayWebWalletData from a dict
samsung_pay_web_wallet_data_from_dict = SamsungPayWebWalletData.from_dict(samsung_pay_web_wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


