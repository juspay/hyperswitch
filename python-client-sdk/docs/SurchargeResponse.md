# SurchargeResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**type** | **str** |  | 
**value** | [**SurchargePercentage**](SurchargePercentage.md) |  | 

## Example

```python
from hyperswitch.models.surcharge_response import SurchargeResponse

# TODO update the JSON string below
json = "{}"
# create an instance of SurchargeResponse from a JSON string
surcharge_response_instance = SurchargeResponse.from_json(json)
# print the JSON string representation of the object
print(SurchargeResponse.to_json())

# convert the object into a dict
surcharge_response_dict = surcharge_response_instance.to_dict()
# create an instance of SurchargeResponse from a dict
surcharge_response_from_dict = SurchargeResponse.from_dict(surcharge_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


