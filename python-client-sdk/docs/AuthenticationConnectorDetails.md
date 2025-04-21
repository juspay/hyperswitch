# AuthenticationConnectorDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**authentication_connectors** | [**List[AuthenticationConnectors]**](AuthenticationConnectors.md) | List of authentication connectors | 
**three_ds_requestor_url** | **str** | URL of the (customer service) website that will be shown to the shopper in case of technical errors during the 3D Secure 2 process. | 
**three_ds_requestor_app_url** | **str** | Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred. | [optional] 

## Example

```python
from hyperswitch.models.authentication_connector_details import AuthenticationConnectorDetails

# TODO update the JSON string below
json = "{}"
# create an instance of AuthenticationConnectorDetails from a JSON string
authentication_connector_details_instance = AuthenticationConnectorDetails.from_json(json)
# print the JSON string representation of the object
print(AuthenticationConnectorDetails.to_json())

# convert the object into a dict
authentication_connector_details_dict = authentication_connector_details_instance.to_dict()
# create an instance of AuthenticationConnectorDetails from a dict
authentication_connector_details_from_dict = AuthenticationConnectorDetails.from_dict(authentication_connector_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


