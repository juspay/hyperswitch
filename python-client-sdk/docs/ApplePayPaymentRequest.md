# ApplePayPaymentRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**country_code** | [**CountryAlpha2**](CountryAlpha2.md) |  | 
**currency_code** | [**Currency**](Currency.md) |  | 
**total** | [**AmountInfo**](AmountInfo.md) |  | 
**merchant_capabilities** | **List[str]** | The list of merchant capabilities(ex: whether capable of 3ds or no-3ds) | [optional] 
**supported_networks** | **List[str]** | The list of supported networks | [optional] 
**merchant_identifier** | **str** |  | [optional] 
**required_billing_contact_fields** | [**List[ApplePayAddressParameters]**](ApplePayAddressParameters.md) |  | [optional] 
**required_shipping_contact_fields** | [**List[ApplePayAddressParameters]**](ApplePayAddressParameters.md) |  | [optional] 
**recurring_payment_request** | [**ApplePayRecurringPaymentRequest**](ApplePayRecurringPaymentRequest.md) |  | [optional] 

## Example

```python
from hyperswitch.models.apple_pay_payment_request import ApplePayPaymentRequest

# TODO update the JSON string below
json = "{}"
# create an instance of ApplePayPaymentRequest from a JSON string
apple_pay_payment_request_instance = ApplePayPaymentRequest.from_json(json)
# print the JSON string representation of the object
print(ApplePayPaymentRequest.to_json())

# convert the object into a dict
apple_pay_payment_request_dict = apple_pay_payment_request_instance.to_dict()
# create an instance of ApplePayPaymentRequest from a dict
apple_pay_payment_request_from_dict = ApplePayPaymentRequest.from_dict(apple_pay_payment_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


