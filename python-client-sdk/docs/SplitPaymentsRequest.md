# SplitPaymentsRequest

Fee information for Split Payments to be charged on the payment being collected

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**stripe_split_payment** | [**StripeSplitPaymentRequest**](StripeSplitPaymentRequest.md) |  | 
**adyen_split_payment** | [**AdyenSplitData**](AdyenSplitData.md) |  | 
**xendit_split_payment** | [**XenditSplitRequest**](XenditSplitRequest.md) |  | 

## Example

```python
from hyperswitch.models.split_payments_request import SplitPaymentsRequest

# TODO update the JSON string below
json = "{}"
# create an instance of SplitPaymentsRequest from a JSON string
split_payments_request_instance = SplitPaymentsRequest.from_json(json)
# print the JSON string representation of the object
print(SplitPaymentsRequest.to_json())

# convert the object into a dict
split_payments_request_dict = split_payments_request_instance.to_dict()
# create an instance of SplitPaymentsRequest from a dict
split_payments_request_from_dict = SplitPaymentsRequest.from_dict(split_payments_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


