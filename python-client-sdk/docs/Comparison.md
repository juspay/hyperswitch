# Comparison

Represents a single comparison condition.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**lhs** | **str** | The left hand side which will always be a domain input identifier like \&quot;payment.method.cardtype\&quot; | 
**comparison** | [**ComparisonType**](ComparisonType.md) |  | 
**value** | [**ValueType**](ValueType.md) |  | 
**metadata** | **Dict[str, object]** | Additional metadata that the Static Analyzer and Backend does not touch. This can be used to store useful information for the frontend and is required for communication between the static analyzer and the frontend. | 

## Example

```python
from hyperswitch.models.comparison import Comparison

# TODO update the JSON string below
json = "{}"
# create an instance of Comparison from a JSON string
comparison_instance = Comparison.from_json(json)
# print the JSON string representation of the object
print(Comparison.to_json())

# convert the object into a dict
comparison_dict = comparison_instance.to_dict()
# create an instance of Comparison from a dict
comparison_from_dict = Comparison.from_dict(comparison_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


