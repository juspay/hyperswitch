# ExtendedCardInfoConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**public_key** | **str** | Merchant public key | 
**ttl_in_secs** | **int** | TTL for extended card info | [optional] [default to 900]

## Example

```python
from hyperswitch.models.extended_card_info_config import ExtendedCardInfoConfig

# TODO update the JSON string below
json = "{}"
# create an instance of ExtendedCardInfoConfig from a JSON string
extended_card_info_config_instance = ExtendedCardInfoConfig.from_json(json)
# print the JSON string representation of the object
print(ExtendedCardInfoConfig.to_json())

# convert the object into a dict
extended_card_info_config_dict = extended_card_info_config_instance.to_dict()
# create an instance of ExtendedCardInfoConfig from a dict
extended_card_info_config_from_dict = ExtendedCardInfoConfig.from_dict(extended_card_info_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


