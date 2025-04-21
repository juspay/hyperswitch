# RelayError


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**code** | **str** | The error code | 
**message** | **str** | The error message | 

## Example

```python
from hyperswitch.models.relay_error import RelayError

# TODO update the JSON string below
json = "{}"
# create an instance of RelayError from a JSON string
relay_error_instance = RelayError.from_json(json)
# print the JSON string representation of the object
print(RelayError.to_json())

# convert the object into a dict
relay_error_dict = relay_error_instance.to_dict()
# create an instance of RelayError from a dict
relay_error_from_dict = RelayError.from_dict(relay_error_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


