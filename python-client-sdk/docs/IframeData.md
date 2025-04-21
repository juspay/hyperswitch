# IframeData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds_method_url** | **str** | ThreeDS method url | 
**three_ds_method_data_submission** | **bool** | Whether ThreeDS method data submission is required | 
**three_ds_method_data** | **str** | ThreeDS method data | [optional] 
**directory_server_id** | **str** | ThreeDS Server ID | 
**message_version** | **str** | ThreeDS Protocol version | [optional] 
**method_key** | **str** |  | 

## Example

```python
from hyperswitch.models.iframe_data import IframeData

# TODO update the JSON string below
json = "{}"
# create an instance of IframeData from a JSON string
iframe_data_instance = IframeData.from_json(json)
# print the JSON string representation of the object
print(IframeData.to_json())

# convert the object into a dict
iframe_data_dict = iframe_data_instance.to_dict()
# create an instance of IframeData from a dict
iframe_data_from_dict = IframeData.from_dict(iframe_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


