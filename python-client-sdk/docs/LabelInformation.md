# LabelInformation


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**label** | **str** |  | 
**target_count** | **int** |  | 
**target_time** | **int** |  | 
**mca_id** | **str** |  | 

## Example

```python
from hyperswitch.models.label_information import LabelInformation

# TODO update the JSON string below
json = "{}"
# create an instance of LabelInformation from a JSON string
label_information_instance = LabelInformation.from_json(json)
# print the JSON string representation of the object
print(LabelInformation.to_json())

# convert the object into a dict
label_information_dict = label_information_instance.to_dict()
# create an instance of LabelInformation from a dict
label_information_from_dict = LabelInformation.from_dict(label_information_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


