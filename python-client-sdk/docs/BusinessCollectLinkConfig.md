# BusinessCollectLinkConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**logo** | **str** | Merchant&#39;s display logo | [optional] 
**merchant_name** | **str** | Custom merchant name for the link | [optional] 
**theme** | **str** | Primary color to be used in the form represented in hex format | [optional] 
**domain_name** | **str** | Custom domain name to be used for hosting the link | [optional] 
**allowed_domains** | **List[str]** | A list of allowed domains (glob patterns) where this link can be embedded / opened from | 
**enabled_payment_methods** | [**List[EnabledPaymentMethod]**](EnabledPaymentMethod.md) | List of payment methods shown on collect UI | 

## Example

```python
from hyperswitch.models.business_collect_link_config import BusinessCollectLinkConfig

# TODO update the JSON string below
json = "{}"
# create an instance of BusinessCollectLinkConfig from a JSON string
business_collect_link_config_instance = BusinessCollectLinkConfig.from_json(json)
# print the JSON string representation of the object
print(BusinessCollectLinkConfig.to_json())

# convert the object into a dict
business_collect_link_config_dict = business_collect_link_config_instance.to_dict()
# create an instance of BusinessCollectLinkConfig from a dict
business_collect_link_config_from_dict = BusinessCollectLinkConfig.from_dict(business_collect_link_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


