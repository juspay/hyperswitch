# ApplePayRegularBillingRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**amount** | **str** | The amount of the recurring payment | 
**label** | **str** | The label that Apple Pay displays to the user in the payment sheet with the recurring details | 
**payment_timing** | [**ApplePayPaymentTiming**](ApplePayPaymentTiming.md) |  | 
**recurring_payment_start_date** | **datetime** | The date of the first payment | [optional] 
**recurring_payment_end_date** | **datetime** | The date of the final payment | [optional] 
**recurring_payment_interval_unit** | [**RecurringPaymentIntervalUnit**](RecurringPaymentIntervalUnit.md) |  | [optional] 
**recurring_payment_interval_count** | **int** | The number of interval units that make up the total payment interval | [optional] 

## Example

```python
from hyperswitch.models.apple_pay_regular_billing_request import ApplePayRegularBillingRequest

# TODO update the JSON string below
json = "{}"
# create an instance of ApplePayRegularBillingRequest from a JSON string
apple_pay_regular_billing_request_instance = ApplePayRegularBillingRequest.from_json(json)
# print the JSON string representation of the object
print(ApplePayRegularBillingRequest.to_json())

# convert the object into a dict
apple_pay_regular_billing_request_dict = apple_pay_regular_billing_request_instance.to_dict()
# create an instance of ApplePayRegularBillingRequest from a dict
apple_pay_regular_billing_request_from_dict = ApplePayRegularBillingRequest.from_dict(apple_pay_regular_billing_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


