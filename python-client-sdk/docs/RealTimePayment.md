# RealTimePayment


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**real_time_payment** | [**RealTimePaymentData**](RealTimePaymentData.md) |  | 

## Example

```python
from hyperswitch.models.real_time_payment import RealTimePayment

# TODO update the JSON string below
json = "{}"
# create an instance of RealTimePayment from a JSON string
real_time_payment_instance = RealTimePayment.from_json(json)
# print the JSON string representation of the object
print(RealTimePayment.to_json())

# convert the object into a dict
real_time_payment_dict = real_time_payment_instance.to_dict()
# create an instance of RealTimePayment from a dict
real_time_payment_from_dict = RealTimePayment.from_dict(real_time_payment_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


