# MerchantConnectorWebhookDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_secret** | **str** |  | 
**additional_secret** | **str** |  | 

## Example

```python
from hyperswitch.models.merchant_connector_webhook_details import MerchantConnectorWebhookDetails

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantConnectorWebhookDetails from a JSON string
merchant_connector_webhook_details_instance = MerchantConnectorWebhookDetails.from_json(json)
# print the JSON string representation of the object
print(MerchantConnectorWebhookDetails.to_json())

# convert the object into a dict
merchant_connector_webhook_details_dict = merchant_connector_webhook_details_instance.to_dict()
# create an instance of MerchantConnectorWebhookDetails from a dict
merchant_connector_webhook_details_from_dict = MerchantConnectorWebhookDetails.from_dict(merchant_connector_webhook_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


