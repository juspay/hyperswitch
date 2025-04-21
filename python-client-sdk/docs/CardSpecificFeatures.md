# CardSpecificFeatures


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**three_ds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**no_three_ds** | [**FeatureStatus**](FeatureStatus.md) |  | 
**supported_card_networks** | [**List[CardNetwork]**](CardNetwork.md) | List of supported card networks | 

## Example

```python
from hyperswitch.models.card_specific_features import CardSpecificFeatures

# TODO update the JSON string below
json = "{}"
# create an instance of CardSpecificFeatures from a JSON string
card_specific_features_instance = CardSpecificFeatures.from_json(json)
# print the JSON string representation of the object
print(CardSpecificFeatures.to_json())

# convert the object into a dict
card_specific_features_dict = card_specific_features_instance.to_dict()
# create an instance of CardSpecificFeatures from a dict
card_specific_features_from_dict = CardSpecificFeatures.from_dict(card_specific_features_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


