# ConnectorWalletDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**apple_pay_combined** | **object** | This field contains the Apple Pay certificates and credentials for iOS and Web Apple Pay flow | [optional] 
**apple_pay** | **object** | This field is for our legacy Apple Pay flow that contains the Apple Pay certificates and credentials for only iOS Apple Pay flow | [optional] 
**samsung_pay** | **object** | This field contains the Samsung Pay certificates and credentials | [optional] 
**paze** | **object** | This field contains the Paze certificates and credentials | [optional] 
**google_pay** | **object** | This field contains the Google Pay certificates and credentials | [optional] 

## Example

```python
from hyperswitch.models.connector_wallet_details import ConnectorWalletDetails

# TODO update the JSON string below
json = "{}"
# create an instance of ConnectorWalletDetails from a JSON string
connector_wallet_details_instance = ConnectorWalletDetails.from_json(json)
# print the JSON string representation of the object
print(ConnectorWalletDetails.to_json())

# convert the object into a dict
connector_wallet_details_dict = connector_wallet_details_instance.to_dict()
# create an instance of ConnectorWalletDetails from a dict
connector_wallet_details_from_dict = ConnectorWalletDetails.from_dict(connector_wallet_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


