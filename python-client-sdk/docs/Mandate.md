# Mandate


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**mandate_id** | **str** | Mandate identifier | [optional] 
**status** | **str** | Mandate status | [optional] 

## Example

```python
from hyperswitch.models.mandate import Mandate

# TODO update the JSON string below
json = "{}"
# create an instance of Mandate from a JSON string
mandate_instance = Mandate.from_json(json)
# print the JSON string representation of the object
print(Mandate.to_json())

# convert the object into a dict
mandate_dict = mandate_instance.to_dict()
# create an instance of Mandate from a dict
mandate_from_dict = Mandate.from_dict(mandate_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


