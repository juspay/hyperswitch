# ConnectorVolumeSplit


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | [**RoutableConnectorChoice**](RoutableConnectorChoice.md) |  | 
**split** | **int** |  | 

## Example

```python
from hyperswitch.models.connector_volume_split import ConnectorVolumeSplit

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorVolumeSplit from a JSON string
connector_volume_split_instance = ConnectorVolumeSplit.from_json(json)
# print the JSON string representation of the object
print(ConnectorVolumeSplit.to_json())

# convert the object into a dict
connector_volume_split_dict = connector_volume_split_instance.to_dict()
# create an instance of ConnectorVolumeSplit from a dict
connector_volume_split_from_dict = ConnectorVolumeSplit.from_dict(connector_volume_split_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


