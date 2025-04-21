# VolumeSplit


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | [**List[ConnectorVolumeSplit]**](ConnectorVolumeSplit.md) |  | 

## Example

```python
from hyperswitch.models.volume_split import VolumeSplit

# TODO update the JSON string below
json = "{}"
# create an instance of VolumeSplit from a JSON string
volume_split_instance = VolumeSplit.from_json(json)
# print the JSON string representation of the object
print(VolumeSplit.to_json())

# convert the object into a dict
volume_split_dict = volume_split_instance.to_dict()
# create an instance of VolumeSplit from a dict
volume_split_from_dict = VolumeSplit.from_dict(volume_split_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


