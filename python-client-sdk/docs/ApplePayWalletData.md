# ApplePayWalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_data** | **str** | The payment data of Apple pay | 
**payment_method** | [**ApplepayPaymentMethod**](ApplepayPaymentMethod.md) |  | 
**transaction_identifier** | **str** | The unique identifier for the transaction | 

## Example

```python
from hyperswitch.models.apple_pay_wallet_data import ApplePayWalletData

# TODO update the JSON string below
json = "{}"
# create an instance of ApplePayWalletData from a JSON string
apple_pay_wallet_data_instance = ApplePayWalletData.from_json(json)
# print the JSON string representation of the object
print(ApplePayWalletData.to_json())

# convert the object into a dict
apple_pay_wallet_data_dict = apple_pay_wallet_data_instance.to_dict()
# create an instance of ApplePayWalletData from a dict
apple_pay_wallet_data_from_dict = ApplePayWalletData.from_dict(apple_pay_wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


