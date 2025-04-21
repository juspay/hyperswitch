# OrganizationResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**organization_id** | **str** | The unique identifier for the Organization | 
**organization_name** | **str** | Name of the Organization | [optional] 
**organization_details** | **object** | Details about the organization | [optional] 
**metadata** | **object** | Metadata is useful for storing additional, unstructured information on an object. | [optional] 
**modified_at** | **datetime** |  | 
**created_at** | **datetime** |  | 

## Example

```python
from hyperswitch.models.organization_response import OrganizationResponse

# TODO update the JSON string below
json = "{}"
# create an instance of OrganizationResponse from a JSON string
organization_response_instance = OrganizationResponse.from_json(json)
# print the JSON string representation of the object
print(OrganizationResponse.to_json())

# convert the object into a dict
organization_response_dict = organization_response_instance.to_dict()
# create an instance of OrganizationResponse from a dict
organization_response_from_dict = OrganizationResponse.from_dict(organization_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


