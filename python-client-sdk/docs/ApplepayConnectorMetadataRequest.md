# ApplepayConnectorMetadataRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**session_token_data** | [**SessionTokenInfo**](SessionTokenInfo.md) |  | [optional] 

## Example

```python
from hyperswitch.models.applepay_connector_metadata_request import ApplepayConnectorMetadataRequest

# TODO update the JSON string below
json = "{}"
# create an instance of ApplepayConnectorMetadataRequest from a JSON string
applepay_connector_metadata_request_instance = ApplepayConnectorMetadataRequest.from_json(json)
# print the JSON string representation of the object
print(ApplepayConnectorMetadataRequest.to_json())

# convert the object into a dict
applepay_connector_metadata_request_dict = applepay_connector_metadata_request_instance.to_dict()
# create an instance of ApplepayConnectorMetadataRequest from a dict
applepay_connector_metadata_request_from_dict = ApplepayConnectorMetadataRequest.from_dict(applepay_connector_metadata_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


