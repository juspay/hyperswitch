# OrganizationUpdateRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**organization_name** | **str** | Name of the organization | [optional] 
**organization_details** | **object** | Details about the organization | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 

## Example

```python
from hyperswitch.models.organization_update_request import OrganizationUpdateRequest

# TODO update the JSON string below
json = "{}"
# create an instance of OrganizationUpdateRequest from a JSON string
organization_update_request_instance = OrganizationUpdateRequest.from_json(json)
# print the JSON string representation of the object
print(OrganizationUpdateRequest.to_json())

# convert the object into a dict
organization_update_request_dict = organization_update_request_instance.to_dict()
# create an instance of OrganizationUpdateRequest from a dict
organization_update_request_from_dict = OrganizationUpdateRequest.from_dict(organization_update_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


