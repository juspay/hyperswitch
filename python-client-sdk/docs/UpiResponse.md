# UpiResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**upi_collect** | [**UpiCollectAdditionalData**](UpiCollectAdditionalData.md) |  | 
**upi_intent** | **object** |  | 

## Example

```python
from hyperswitch.models.upi_response import UpiResponse

# TODO update the JSON string below
json = "{}"
# create an instance of UpiResponse from a JSON string
upi_response_instance = UpiResponse.from_json(json)
# print the JSON string representation of the object
print(UpiResponse.to_json())

# convert the object into a dict
upi_response_dict = upi_response_instance.to_dict()
# create an instance of UpiResponse from a dict
upi_response_from_dict = UpiResponse.from_dict(upi_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


