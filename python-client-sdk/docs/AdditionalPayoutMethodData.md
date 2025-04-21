# AdditionalPayoutMethodData

Masked payout method details for storing in db

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardAdditionalData**](CardAdditionalData.md) |  | 
**bank** | [**BankAdditionalData**](BankAdditionalData.md) |  | 
**wallet** | [**WalletAdditionalData**](WalletAdditionalData.md) |  | 

## Example

```python
from hyperswitch.models.additional_payout_method_data import AdditionalPayoutMethodData

# TODO update the JSON string below
json = "{}"
# create an instance of AdditionalPayoutMethodData from a JSON string
additional_payout_method_data_instance = AdditionalPayoutMethodData.from_json(json)
# print the JSON string representation of the object
print(AdditionalPayoutMethodData.to_json())

# convert the object into a dict
additional_payout_method_data_dict = additional_payout_method_data_instance.to_dict()
# create an instance of AdditionalPayoutMethodData from a dict
additional_payout_method_data_from_dict = AdditionalPayoutMethodData.from_dict(additional_payout_method_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


