# MandateRevokedResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**mandate_id** | **str** | The identifier for mandate | 
**status** | [**MandateStatus**](MandateStatus.md) |  | 
**error_code** | **str** | If there was an error while calling the connectors the code is received here | [optional] 
**error_message** | **str** | If there was an error while calling the connector the error message is received here | [optional] 

## Example

```python
from hyperswitch.models.mandate_revoked_response import MandateRevokedResponse

# TODO update the JSON string below
json = "{}"
# create an instance of MandateRevokedResponse from a JSON string
mandate_revoked_response_instance = MandateRevokedResponse.from_json(json)
# print the JSON string representation of the object
print(MandateRevokedResponse.to_json())

# convert the object into a dict
mandate_revoked_response_dict = mandate_revoked_response_instance.to_dict()
# create an instance of MandateRevokedResponse from a dict
mandate_revoked_response_from_dict = MandateRevokedResponse.from_dict(mandate_revoked_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


