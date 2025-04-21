# GsmRetrieveRequest


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**connector** | [**Connector**](Connector.md) |  | 
**flow** | **str** | The flow in which the code and message occurred for a connector | 
**sub_flow** | **str** | The sub_flow in which the code and message occurred  for a connector | 
**code** | **str** | code received from the connector | 
**message** | **str** | message received from the connector | 

## Example

```python
from hyperswitch.models.gsm_retrieve_request import GsmRetrieveRequest

# TODO update the JSON string below
json = "{}"
# create an instance of GsmRetrieveRequest from a JSON string
gsm_retrieve_request_instance = GsmRetrieveRequest.from_json(json)
# print the JSON string representation of the object
print(GsmRetrieveRequest.to_json())

# convert the object into a dict
gsm_retrieve_request_dict = gsm_retrieve_request_instance.to_dict()
# create an instance of GsmRetrieveRequest from a dict
gsm_retrieve_request_from_dict = GsmRetrieveRequest.from_dict(gsm_retrieve_request_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


