# PayoutMethodData

The payout method information required for carrying out a payout

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**card** | [**CardPayout**](CardPayout.md) |  | 
**bank** | [**Bank**](Bank.md) |  | 
**wallet** | [**Wallet**](Wallet.md) |  | 

## Example

```python
from hyperswitch.models.payout_method_data import PayoutMethodData

# TODO update the JSON string below
json = "{}"
# create an instance of PayoutMethodData from a JSON string
payout_method_data_instance = PayoutMethodData.from_json(json)
# print the JSON string representation of the object
print(PayoutMethodData.to_json())

# convert the object into a dict
payout_method_data_dict = payout_method_data_instance.to_dict()
# create an instance of PayoutMethodData from a dict
payout_method_data_from_dict = PayoutMethodData.from_dict(payout_method_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


