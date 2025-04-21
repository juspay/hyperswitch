# ConnectorSelectionOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**data** | [**List[RoutableConnectorChoice]**](RoutableConnectorChoice.md) |  | 

## Example

```python
from hyperswitch.models.connector_selection_one_of import ConnectorSelectionOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorSelectionOneOf from a JSON string
connector_selection_one_of_instance = ConnectorSelectionOneOf.from_json(json)
# print the JSON string representation of the object
print(ConnectorSelectionOneOf.to_json())

# convert the object into a dict
connector_selection_one_of_dict = connector_selection_one_of_instance.to_dict()
# create an instance of ConnectorSelectionOneOf from a dict
connector_selection_one_of_from_dict = ConnectorSelectionOneOf.from_dict(connector_selection_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


