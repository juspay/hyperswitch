# SurchargeResponseOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**value** | **int** | This Unit struct represents MinorUnit in which core amount works | 

## Example

```python
from hyperswitch.models.surcharge_response_one_of import SurchargeResponseOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of SurchargeResponseOneOf from a JSON string
surcharge_response_one_of_instance = SurchargeResponseOneOf.from_json(json)
# print the JSON string representation of the object
print(SurchargeResponseOneOf.to_json())

# convert the object into a dict
surcharge_response_one_of_dict = surcharge_response_one_of_instance.to_dict()
# create an instance of SurchargeResponseOneOf from a dict
surcharge_response_one_of_from_dict = SurchargeResponseOneOf.from_dict(surcharge_response_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


