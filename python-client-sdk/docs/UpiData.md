# UpiData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**upi_collect** | [**UpiCollectData**](UpiCollectData.md) |  | 
**upi_intent** | **object** |  | 

## Example

```python
from hyperswitch.models.upi_data import UpiData

# TODO update the JSON string below
json = "{}"
# create an instance of UpiData from a JSON string
upi_data_instance = UpiData.from_json(json)
# print the JSON string representation of the object
print(UpiData.to_json())

# convert the object into a dict
upi_data_dict = upi_data_instance.to_dict()
# create an instance of UpiData from a dict
upi_data_from_dict = UpiData.from_dict(upi_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


