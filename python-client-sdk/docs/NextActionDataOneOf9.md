# NextActionDataOneOf9

Contains consent to collect otp for mobile payment

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**consent_data_required** | [**MobilePaymentConsent**](MobilePaymentConsent.md) |  | 
**type** | **str** |  | 

## Example

```python
from hyperswitch.models.next_action_data_one_of9 import NextActionDataOneOf9

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionDataOneOf9 from a JSON string
next_action_data_one_of9_instance = NextActionDataOneOf9.from_json(json)
# print the JSON string representation of the object
print(NextActionDataOneOf9.to_json())

# convert the object into a dict
next_action_data_one_of9_dict = next_action_data_one_of9_instance.to_dict()
# create an instance of NextActionDataOneOf9 from a dict
next_action_data_one_of9_from_dict = NextActionDataOneOf9.from_dict(next_action_data_one_of9_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


