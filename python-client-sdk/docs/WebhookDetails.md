# WebhookDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**webhook_version** | **str** | The version for Webhook | [optional] 
**webhook_username** | **str** | The user name for Webhook login | [optional] 
**webhook_password** | **str** | The password for Webhook login | [optional] 
**webhook_url** | **str** | The url for the webhook endpoint | [optional] 
**payment_created_enabled** | **bool** | If this property is true, a webhook message is posted whenever a new payment is created | [optional] 
**payment_succeeded_enabled** | **bool** | If this property is true, a webhook message is posted whenever a payment is successful | [optional] 
**payment_failed_enabled** | **bool** | If this property is true, a webhook message is posted whenever a payment fails | [optional] 

## Example

```python
from hyperswitch.models.webhook_details import WebhookDetails

# TODO update the JSON string below
json = "{}"
# create an instance of WebhookDetails from a JSON string
webhook_details_instance = WebhookDetails.from_json(json)
# print the JSON string representation of the object
print(WebhookDetails.to_json())

# convert the object into a dict
webhook_details_dict = webhook_details_instance.to_dict()
# create an instance of WebhookDetails from a dict
webhook_details_from_dict = WebhookDetails.from_dict(webhook_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


