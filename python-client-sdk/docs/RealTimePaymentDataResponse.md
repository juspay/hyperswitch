# RealTimePaymentDataResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**fps** | **object** |  | 
**duit_now** | **object** |  | 
**prompt_pay** | **object** |  | 
**viet_qr** | **object** |  | 

## Example

```python
from hyperswitch.models.real_time_payment_data_response import RealTimePaymentDataResponse

# TODO update the JSON string below
json = "{}"
# create an instance of RealTimePaymentDataResponse from a JSON string
real_time_payment_data_response_instance = RealTimePaymentDataResponse.from_json(json)
# print the JSON string representation of the object
print(RealTimePaymentDataResponse.to_json())

# convert the object into a dict
real_time_payment_data_response_dict = real_time_payment_data_response_instance.to_dict()
# create an instance of RealTimePaymentDataResponse from a dict
real_time_payment_data_response_from_dict = RealTimePaymentDataResponse.from_dict(real_time_payment_data_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


