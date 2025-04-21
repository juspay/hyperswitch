# XenditChargeResponseData

Charge Information

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**multiple_splits** | [**XenditMultipleSplitResponse**](XenditMultipleSplitResponse.md) |  | 
**single_split** | [**XenditSplitSubMerchantData**](XenditSplitSubMerchantData.md) |  | 

## Example

```python
from hyperswitch.models.xendit_charge_response_data import XenditChargeResponseData

# TODO update the JSON string below
json = "{}"
# create an instance of XenditChargeResponseData from a JSON string
xendit_charge_response_data_instance = XenditChargeResponseData.from_json(json)
# print the JSON string representation of the object
print(XenditChargeResponseData.to_json())

# convert the object into a dict
xendit_charge_response_data_dict = xendit_charge_response_data_instance.to_dict()
# create an instance of XenditChargeResponseData from a dict
xendit_charge_response_data_from_dict = XenditChargeResponseData.from_dict(xendit_charge_response_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


