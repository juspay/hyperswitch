# BusinessPayoutLinkConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**logo** | **str** | Merchant&#39;s display logo | [optional] 
**merchant_name** | **str** | Custom merchant name for the link | [optional] 
**theme** | **str** | Primary color to be used in the form represented in hex format | [optional] 
**domain_name** | **str** | Custom domain name to be used for hosting the link | [optional] 
**allowed_domains** | **List[str]** | A list of allowed domains (glob patterns) where this link can be embedded / opened from | 
**form_layout** | [**UIWidgetFormLayout**](UIWidgetFormLayout.md) |  | [optional] 
**payout_test_mode** | **bool** | Allows for removing any validations / pre-requisites which are necessary in a production environment | [optional] [default to False]

## Example

```python
from hyperswitch.models.business_payout_link_config import BusinessPayoutLinkConfig

# TODO update the JSON string below
json = "{}"
# create an instance of BusinessPayoutLinkConfig from a JSON string
business_payout_link_config_instance = BusinessPayoutLinkConfig.from_json(json)
# print the JSON string representation of the object
print(BusinessPayoutLinkConfig.to_json())

# convert the object into a dict
business_payout_link_config_dict = business_payout_link_config_instance.to_dict()
# create an instance of BusinessPayoutLinkConfig from a dict
business_payout_link_config_from_dict = BusinessPayoutLinkConfig.from_dict(business_payout_link_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


