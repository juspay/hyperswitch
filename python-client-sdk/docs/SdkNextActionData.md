# SdkNextActionData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**next_action** | [**NextActionCall**](NextActionCall.md) |  | 
**order_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.sdk_next_action_data import SdkNextActionData

# TODO update the JSON string below
json = "{}"
# create an instance of SdkNextActionData from a JSON string
sdk_next_action_data_instance = SdkNextActionData.from_json(json)
# print the JSON string representation of the object
print(SdkNextActionData.to_json())

# convert the object into a dict
sdk_next_action_data_dict = sdk_next_action_data_instance.to_dict()
# create an instance of SdkNextActionData from a dict
sdk_next_action_data_from_dict = SdkNextActionData.from_dict(sdk_next_action_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


