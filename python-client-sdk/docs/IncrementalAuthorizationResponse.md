# IncrementalAuthorizationResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authorization_id** | **str** | The unique identifier of authorization | 
**amount** | **int** | Amount the authorization has been made for | 
**status** | [**AuthorizationStatus**](AuthorizationStatus.md) |  | 
**error_code** | **str** | Error code sent by the connector for authorization | [optional] 
**error_message** | **str** | Error message sent by the connector for authorization | [optional] 
**previously_authorized_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | 

## Example

```python
from hyperswitch.models.incremental_authorization_response import IncrementalAuthorizationResponse

# TODO update the JSON string below
json = "{}"
# create an instance of IncrementalAuthorizationResponse from a JSON string
incremental_authorization_response_instance = IncrementalAuthorizationResponse.from_json(json)
# print the JSON string representation of the object
print(IncrementalAuthorizationResponse.to_json())

# convert the object into a dict
incremental_authorization_response_dict = incremental_authorization_response_instance.to_dict()
# create an instance of IncrementalAuthorizationResponse from a dict
incremental_authorization_response_from_dict = IncrementalAuthorizationResponse.from_dict(incremental_authorization_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


