# MobilePaymentNextStepData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**consent_data_required** | [**MobilePaymentConsent**](MobilePaymentConsent.md) |  | 

## Example

```python
from hyperswitch.models.mobile_payment_next_step_data import MobilePaymentNextStepData

# TODO update the JSON string below
json = "{}"
# create an instance of MobilePaymentNextStepData from a JSON string
mobile_payment_next_step_data_instance = MobilePaymentNextStepData.from_json(json)
# print the JSON string representation of the object
print(MobilePaymentNextStepData.to_json())

# convert the object into a dict
mobile_payment_next_step_data_dict = mobile_payment_next_step_data_instance.to_dict()
# create an instance of MobilePaymentNextStepData from a dict
mobile_payment_next_step_data_from_dict = MobilePaymentNextStepData.from_dict(mobile_payment_next_step_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


