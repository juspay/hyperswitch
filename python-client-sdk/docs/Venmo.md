# Venmo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**telephone_number** | **str** | mobile number linked to venmo account | 

## Example

```python
from hyperswitch.models.venmo import Venmo

# TODO update the JSON string below
json = "{}"
# create an instance of Venmo from a JSON string
venmo_instance = Venmo.from_json(json)
# print the JSON string representation of the object
print(Venmo.to_json())

# convert the object into a dict
venmo_dict = venmo_instance.to_dict()
# create an instance of Venmo from a dict
venmo_from_dict = Venmo.from_dict(venmo_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


