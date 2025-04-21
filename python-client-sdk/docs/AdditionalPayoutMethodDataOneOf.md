# AdditionalPayoutMethodDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardAdditionalData**](CardAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.additional_payout_method_data_one_of import AdditionalPayoutMethodDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of AdditionalPayoutMethodDataOneOf from a JSON string
additional_payout_method_data_one_of_instance = AdditionalPayoutMethodDataOneOf.from_json(json)
# print the JSON string representation of the object
print(AdditionalPayoutMethodDataOneOf.to_json())

# convert the object into a dict
additional_payout_method_data_one_of_dict = additional_payout_method_data_one_of_instance.to_dict()
# create an instance of AdditionalPayoutMethodDataOneOf from a dict
additional_payout_method_data_one_of_from_dict = AdditionalPayoutMethodDataOneOf.from_dict(additional_payout_method_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


