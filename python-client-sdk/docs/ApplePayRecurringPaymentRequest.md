# ApplePayRecurringPaymentRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payment_description** | **str** | A description of the recurring payment that Apple Pay displays to the user in the payment sheet | 
**regular_billing** | [**ApplePayRegularBillingRequest**](ApplePayRegularBillingRequest.md) |  | 
**billing_agreement** | **str** | A localized billing agreement that the payment sheet displays to the user before the user authorizes the payment | [optional] 
**management_u_r_l** | **str** | A URL to a web page where the user can update or delete the payment method for the recurring payment | 

## Example

```python
from hyperswitch.models.apple_pay_recurring_payment_request import ApplePayRecurringPaymentRequest

# TODO update the JSON string below
json = "{}"
# create an instance of ApplePayRecurringPaymentRequest from a JSON string
apple_pay_recurring_payment_request_instance = ApplePayRecurringPaymentRequest.from_json(json)
# print the JSON string representation of the object
print(ApplePayRecurringPaymentRequest.to_json())

# convert the object into a dict
apple_pay_recurring_payment_request_dict = apple_pay_recurring_payment_request_instance.to_dict()
# create an instance of ApplePayRecurringPaymentRequest from a dict
apple_pay_recurring_payment_request_from_dict = ApplePayRecurringPaymentRequest.from_dict(apple_pay_recurring_payment_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


