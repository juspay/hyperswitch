# PayoutMethodDataResponseOneOf2


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**wallet** | [**WalletAdditionalData**](WalletAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.payout_method_data_response_one_of2 import PayoutMethodDataResponseOneOf2

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutMethodDataResponseOneOf2 from a JSON string
payout_method_data_response_one_of2_instance = PayoutMethodDataResponseOneOf2.from_json(json)
# print the JSON string representation of the object
print(PayoutMethodDataResponseOneOf2.to_json())

# convert the object into a dict
payout_method_data_response_one_of2_dict = payout_method_data_response_one_of2_instance.to_dict()
# create an instance of PayoutMethodDataResponseOneOf2 from a dict
payout_method_data_response_one_of2_from_dict = PayoutMethodDataResponseOneOf2.from_dict(payout_method_data_response_one_of2_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


