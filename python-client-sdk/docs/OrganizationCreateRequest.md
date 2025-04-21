# OrganizationCreateRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**organization_name** | **str** | Name of the organization | 
**organization_details** | **object** | Details about the organization | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 

## Example

```python
from hyperswitch.models.organization_create_request import OrganizationCreateRequest

# TODO update the JSON string below
json = "{}"
# create an instance of OrganizationCreateRequest from a JSON string
organization_create_request_instance = OrganizationCreateRequest.from_json(json)
# print the JSON string representation of the object
print(OrganizationCreateRequest.to_json())

# convert the object into a dict
organization_create_request_dict = organization_create_request_instance.to_dict()
# create an instance of OrganizationCreateRequest from a dict
organization_create_request_from_dict = OrganizationCreateRequest.from_dict(organization_create_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


