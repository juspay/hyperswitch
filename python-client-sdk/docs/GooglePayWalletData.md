# GooglePayWalletData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** | The type of payment method | 
**description** | **str** | User-facing message to describe the payment method that funds this transaction. | 
**info** | [**GooglePayPaymentMethodInfo**](GooglePayPaymentMethodInfo.md) |  | 
**tokenization_data** | [**GpayTokenizationData**](GpayTokenizationData.md) |  | 

## Example

```python
from hyperswitch.models.google_pay_wallet_data import GooglePayWalletData

# TODO update the JSON string below
json = "{}"
# create an instance of GooglePayWalletData from a JSON string
google_pay_wallet_data_instance = GooglePayWalletData.from_json(json)
# print the JSON string representation of the object
print(GooglePayWalletData.to_json())

# convert the object into a dict
google_pay_wallet_data_dict = google_pay_wallet_data_instance.to_dict()
# create an instance of GooglePayWalletData from a dict
google_pay_wallet_data_from_dict = GooglePayWalletData.from_dict(google_pay_wallet_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


