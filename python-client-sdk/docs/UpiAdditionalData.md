# UpiAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**upi_collect** | [**UpiCollectAdditionalData**](UpiCollectAdditionalData.md) |  | 
**upi_intent** | **object** |  | 

## Example

```python
from hyperswitch.models.upi_additional_data import UpiAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of UpiAdditionalData from a JSON string
upi_additional_data_instance = UpiAdditionalData.from_json(json)
# print the JSON string representation of the object
print(UpiAdditionalData.to_json())

# convert the object into a dict
upi_additional_data_dict = upi_additional_data_instance.to_dict()
# create an instance of UpiAdditionalData from a dict
upi_additional_data_from_dict = UpiAdditionalData.from_dict(upi_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


