# GenericLinkUiConfig

Object for GenericLinkUiConfig

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**logo** | **str** | Merchant&#39;s display logo | [optional] 
**merchant_name** | **str** | Custom merchant name for the link | [optional] 
**theme** | **str** | Primary color to be used in the form represented in hex format | [optional] 

## Example

```python
from hyperswitch.models.generic_link_ui_config import GenericLinkUiConfig

# TODO update the JSON string below
json = "{}"
# create an instance of GenericLinkUiConfig from a JSON string
generic_link_ui_config_instance = GenericLinkUiConfig.from_json(json)
# print the JSON string representation of the object
print(GenericLinkUiConfig.to_json())

# convert the object into a dict
generic_link_ui_config_dict = generic_link_ui_config_instance.to_dict()
# create an instance of GenericLinkUiConfig from a dict
generic_link_ui_config_from_dict = GenericLinkUiConfig.from_dict(generic_link_ui_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


