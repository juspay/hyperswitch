# StripeChargeResponseData

Fee information to be charged on the payment being collected via Stripe

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**charge_id** | **str** | Identifier for charge created for the payment | [optional] 
**charge_type** | [**PaymentChargeType**](PaymentChargeType.md) |  | 
**application_fees** | **int** | Platform fees collected on the payment | 
**transfer_account_id** | **str** | Identifier for the reseller&#39;s account where the funds were transferred | 

## Example

```python
from hyperswitch.models.stripe_charge_response_data import StripeChargeResponseData

# TODO update the JSON string below
json = "{}"
# create an instance of StripeChargeResponseData from a JSON string
stripe_charge_response_data_instance = StripeChargeResponseData.from_json(json)
# print the JSON string representation of the object
print(StripeChargeResponseData.to_json())

# convert the object into a dict
stripe_charge_response_data_dict = stripe_charge_response_data_instance.to_dict()
# create an instance of StripeChargeResponseData from a dict
stripe_charge_response_data_from_dict = StripeChargeResponseData.from_dict(stripe_charge_response_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


