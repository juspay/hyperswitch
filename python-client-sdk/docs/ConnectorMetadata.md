# ConnectorMetadata

Some connectors like Apple Pay, Airwallex and Noon might require some additional information, find specific details in the child attributes below.

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**apple_pay** | [**ApplepayConnectorMetadataRequest**](ApplepayConnectorMetadataRequest.md) |  | [optional] 
**airwallex** | [**AirwallexData**](AirwallexData.md) |  | [optional] 
**noon** | [**NoonData**](NoonData.md) |  | [optional] 
**braintree** | [**BraintreeData**](BraintreeData.md) |  | [optional] 

## Example

```python
from hyperswitch.models.connector_metadata import ConnectorMetadata

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorMetadata from a JSON string
connector_metadata_instance = ConnectorMetadata.from_json(json)
# print the JSON string representation of the object
print(ConnectorMetadata.to_json())

# convert the object into a dict
connector_metadata_dict = connector_metadata_instance.to_dict()
# create an instance of ConnectorMetadata from a dict
connector_metadata_from_dict = ConnectorMetadata.from_dict(connector_metadata_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


