# RealTimePaymentData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**fps** | **object** |  | 
**duit_now** | **object** |  | 
**prompt_pay** | **object** |  | 
**viet_qr** | **object** |  | 

## Example

```python
from hyperswitch.models.real_time_payment_data import RealTimePaymentData

# TODO update the JSON string below
json = "{}"
# create an instance of RealTimePaymentData from a JSON string
real_time_payment_data_instance = RealTimePaymentData.from_json(json)
# print the JSON string representation of the object
print(RealTimePaymentData.to_json())

# convert the object into a dict
real_time_payment_data_dict = real_time_payment_data_instance.to_dict()
# create an instance of RealTimePaymentData from a dict
real_time_payment_data_from_dict = RealTimePaymentData.from_dict(real_time_payment_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


