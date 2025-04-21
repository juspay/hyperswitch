# AirwallexData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**payload** | **str** | payload required by airwallex | [optional] 

## Example

```python
from hyperswitch.models.airwallex_data import AirwallexData

# TODO update the JSON string below
json = "{}"
# create an instance of AirwallexData from a JSON string
airwallex_data_instance = AirwallexData.from_json(json)
# print the JSON string representation of the object
print(AirwallexData.to_json())

# convert the object into a dict
airwallex_data_dict = airwallex_data_instance.to_dict()
# create an instance of AirwallexData from a dict
airwallex_data_from_dict = AirwallexData.from_dict(airwallex_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


