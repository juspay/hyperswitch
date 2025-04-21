# MobilePayment


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**mobile_payment** | [**MobilePaymentData**](MobilePaymentData.md) |  | 

## Example

```python
from hyperswitch.models.mobile_payment import MobilePayment

# TODO update the JSON string below
json = "{}"
# create an instance of MobilePayment from a JSON string
mobile_payment_instance = MobilePayment.from_json(json)
# print the JSON string representation of the object
print(MobilePayment.to_json())

# convert the object into a dict
mobile_payment_dict = mobile_payment_instance.to_dict()
# create an instance of MobilePayment from a dict
mobile_payment_from_dict = MobilePayment.from_dict(mobile_payment_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


