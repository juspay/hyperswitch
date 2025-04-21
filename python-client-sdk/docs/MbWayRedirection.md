# MbWayRedirection


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**telephone_number** | **str** | Telephone number of the shopper. Should be Portuguese phone number. | 

## Example

```python
from hyperswitch.models.mb_way_redirection import MbWayRedirection

# TODO update the JSON string below
json = "{}"
# create an instance of MbWayRedirection from a JSON string
mb_way_redirection_instance = MbWayRedirection.from_json(json)
# print the JSON string representation of the object
print(MbWayRedirection.to_json())

# convert the object into a dict
mb_way_redirection_dict = mb_way_redirection_instance.to_dict()
# create an instance of MbWayRedirection from a dict
mb_way_redirection_from_dict = MbWayRedirection.from_dict(mb_way_redirection_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


