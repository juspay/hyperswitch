# MandateType


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**single_use** | [**MandateAmountData**](MandateAmountData.md) |  | 
**multi_use** | [**MandateAmountData**](MandateAmountData.md) |  | 

## Example

```python
from hyperswitch.models.mandate_type import MandateType

# TODO update the JSON string below
json = "{}"
# create an instance of MandateType from a JSON string
mandate_type_instance = MandateType.from_json(json)
# print the JSON string representation of the object
print(MandateType.to_json())

# convert the object into a dict
mandate_type_dict = mandate_type_instance.to_dict()
# create an instance of MandateType from a dict
mandate_type_from_dict = MandateType.from_dict(mandate_type_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


