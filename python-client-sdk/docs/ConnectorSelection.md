# ConnectorSelection


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | [**List[ConnectorVolumeSplit]**](ConnectorVolumeSplit.md) |  | 

## Example

```python
from hyperswitch.models.connector_selection import ConnectorSelection

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorSelection from a JSON string
connector_selection_instance = ConnectorSelection.from_json(json)
# print the JSON string representation of the object
print(ConnectorSelection.to_json())

# convert the object into a dict
connector_selection_dict = connector_selection_instance.to_dict()
# create an instance of ConnectorSelection from a dict
connector_selection_from_dict = ConnectorSelection.from_dict(connector_selection_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


