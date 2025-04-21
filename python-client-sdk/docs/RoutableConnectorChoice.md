# RoutableConnectorChoice

Routable Connector chosen for a payment

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | [**RoutableConnectors**](RoutableConnectors.md) |  | 
**merchant_connector_id** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.routable_connector_choice import RoutableConnectorChoice

# TODO update the JSON string below
json = "{}"
# create an instance of RoutableConnectorChoice from a JSON string
routable_connector_choice_instance = RoutableConnectorChoice.from_json(json)
# print the JSON string representation of the object
print(RoutableConnectorChoice.to_json())

# convert the object into a dict
routable_connector_choice_dict = routable_connector_choice_instance.to_dict()
# create an instance of RoutableConnectorChoice from a dict
routable_connector_choice_from_dict = RoutableConnectorChoice.from_dict(routable_connector_choice_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


