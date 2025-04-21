# StripeSplitPaymentRequest

Fee information for Split Payments to be charged on the payment being collected for Stripe

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**charge_type** | [**PaymentChargeType**](PaymentChargeType.md) |  | 
**application_fees** | **int** | Platform fees to be collected on the payment | 
**transfer_account_id** | **str** | Identifier for the reseller&#39;s account where the funds were transferred | 

## Example

```python
from hyperswitch.models.stripe_split_payment_request import StripeSplitPaymentRequest

# TODO update the JSON string below
json = "{}"
# create an instance of StripeSplitPaymentRequest from a JSON string
stripe_split_payment_request_instance = StripeSplitPaymentRequest.from_json(json)
# print the JSON string representation of the object
print(StripeSplitPaymentRequest.to_json())

# convert the object into a dict
stripe_split_payment_request_dict = stripe_split_payment_request_instance.to_dict()
# create an instance of StripeSplitPaymentRequest from a dict
stripe_split_payment_request_from_dict = StripeSplitPaymentRequest.from_dict(stripe_split_payment_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


