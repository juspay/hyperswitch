# FeatureMetadata

additional data that might be required by hyperswitch

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**redirect_response** | [**RedirectResponse**](RedirectResponse.md) |  | [optional] 
**search_tags** | **List[str]** | Additional tags to be used for global search | [optional] 
**apple_pay_recurring_details** | [**ApplePayRecurringDetails**](ApplePayRecurringDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.feature_metadata import FeatureMetadata

# TODO update the JSON string below
json = "{}"
# create an instance of FeatureMetadata from a JSON string
feature_metadata_instance = FeatureMetadata.from_json(json)
# print the JSON string representation of the object
print(FeatureMetadata.to_json())

# convert the object into a dict
feature_metadata_dict = feature_metadata_instance.to_dict()
# create an instance of FeatureMetadata from a dict
feature_metadata_from_dict = FeatureMetadata.from_dict(feature_metadata_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


