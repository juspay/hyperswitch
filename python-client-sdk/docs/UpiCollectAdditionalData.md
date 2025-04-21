# UpiCollectAdditionalData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**vpa_id** | **str** | Masked VPA ID | [optional] 

## Example

```python
from hyperswitch.models.upi_collect_additional_data import UpiCollectAdditionalData

# TODO update the JSON string below
json = "{}"
# create an instance of UpiCollectAdditionalData from a JSON string
upi_collect_additional_data_instance = UpiCollectAdditionalData.from_json(json)
# print the JSON string representation of the object
print(UpiCollectAdditionalData.to_json())

# convert the object into a dict
upi_collect_additional_data_dict = upi_collect_additional_data_instance.to_dict()
# create an instance of UpiCollectAdditionalData from a dict
upi_collect_additional_data_from_dict = UpiCollectAdditionalData.from_dict(upi_collect_additional_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


