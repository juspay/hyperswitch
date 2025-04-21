# ThreeDsData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds_authentication_url** | **str** | ThreeDS authentication url - to initiate authentication | 
**three_ds_authorize_url** | **str** | ThreeDS authorize url - to complete the payment authorization after authentication | 
**three_ds_method_details** | [**ThreeDsMethodData**](ThreeDsMethodData.md) |  | 
**poll_config** | [**PollConfigResponse**](PollConfigResponse.md) |  | 
**message_version** | **str** | Message Version | [optional] 
**directory_server_id** | **str** | Directory Server ID | [optional] 

## Example

```python
from hyperswitch.models.three_ds_data import ThreeDsData

# TODO update the JSON string below
json = "{}"
# create an instance of ThreeDsData from a JSON string
three_ds_data_instance = ThreeDsData.from_json(json)
# print the JSON string representation of the object
print(ThreeDsData.to_json())

# convert the object into a dict
three_ds_data_dict = three_ds_data_instance.to_dict()
# create an instance of ThreeDsData from a dict
three_ds_data_from_dict = ThreeDsData.from_dict(three_ds_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


