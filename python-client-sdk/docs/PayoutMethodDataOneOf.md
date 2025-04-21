# PayoutMethodDataOneOf


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardPayout**](CardPayout.md) |  | 

## Example

```python
from hyperswitch.models.payout_method_data_one_of import PayoutMethodDataOneOf

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutMethodDataOneOf from a JSON string
payout_method_data_one_of_instance = PayoutMethodDataOneOf.from_json(json)
# print the JSON string representation of the object
print(PayoutMethodDataOneOf.to_json())

# convert the object into a dict
payout_method_data_one_of_dict = payout_method_data_one_of_instance.to_dict()
# create an instance of PayoutMethodDataOneOf from a dict
payout_method_data_one_of_from_dict = PayoutMethodDataOneOf.from_dict(payout_method_data_one_of_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


