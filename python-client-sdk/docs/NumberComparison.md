# NumberComparison

Represents a number comparison for \"NumberComparisonArrayValue\"

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**comparison_type** | [**ComparisonType**](ComparisonType.md) |  | 
**number** | **int** | This Unit struct represents MinorUnit in which core amount works | 

## Example

```python
from hyperswitch.models.number_comparison import NumberComparison

# TODO update the JSON string below
json = "{}"
# create an instance of NumberComparison from a JSON string
number_comparison_instance = NumberComparison.from_json(json)
# print the JSON string representation of the object
print(NumberComparison.to_json())

# convert the object into a dict
number_comparison_dict = number_comparison_instance.to_dict()
# create an instance of NumberComparison from a dict
number_comparison_from_dict = NumberComparison.from_dict(number_comparison_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


