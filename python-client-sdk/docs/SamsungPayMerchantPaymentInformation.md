# SamsungPayMerchantPaymentInformation


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**name** | **str** | Merchant name, this will be displayed on the Samsung Pay screen | 
**url** | **str** | Merchant domain that process payments, required for web payments | [optional] 
**country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | 

## Example

```python
from hyperswitch.models.samsung_pay_merchant_payment_information import SamsungPayMerchantPaymentInformation

# TODO update the JSON string below
json = "{}"
# create an instance of SamsungPayMerchantPaymentInformation from a JSON string
samsung_pay_merchant_payment_information_instance = SamsungPayMerchantPaymentInformation.from_json(json)
# print the JSON string representation of the object
print(SamsungPayMerchantPaymentInformation.to_json())

# convert the object into a dict
samsung_pay_merchant_payment_information_dict = samsung_pay_merchant_payment_information_instance.to_dict()
# create an instance of SamsungPayMerchantPaymentInformation from a dict
samsung_pay_merchant_payment_information_from_dict = SamsungPayMerchantPaymentInformation.from_dict(samsung_pay_merchant_payment_information_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


