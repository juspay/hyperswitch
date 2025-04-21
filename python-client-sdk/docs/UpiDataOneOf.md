# UpiDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**upi_collect** | [**UpiCollectData**](UpiCollectData.md) |  | 

## Example

```python
from hyperswitch.models.upi_data_one_of import UpiDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of UpiDataOneOf from a JSON string
upi_data_one_of_instance = UpiDataOneOf.from_json(json)
# print the JSON string representation of the object
print(UpiDataOneOf.to_json())

# convert the object into a dict
upi_data_one_of_dict = upi_data_one_of_instance.to_dict()
# create an instance of UpiDataOneOf from a dict
upi_data_one_of_from_dict = UpiDataOneOf.from_dict(upi_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


