# PayoutMethodDataResponseOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardAdditionalData**](CardAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.payout_method_data_response_one_of import PayoutMethodDataResponseOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutMethodDataResponseOneOf from a JSON string
payout_method_data_response_one_of_instance = PayoutMethodDataResponseOneOf.from_json(json)
# print the JSON string representation of the object
print(PayoutMethodDataResponseOneOf.to_json())

# convert the object into a dict
payout_method_data_response_one_of_dict = payout_method_data_response_one_of_instance.to_dict()
# create an instance of PayoutMethodDataResponseOneOf from a dict
payout_method_data_response_one_of_from_dict = PayoutMethodDataResponseOneOf.from_dict(payout_method_data_response_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


