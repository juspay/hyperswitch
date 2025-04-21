# GpayMerchantInfo


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_id** | **str** | The merchant Identifier that needs to be passed while invoking Gpay SDK | [optional] 
**merchant_name** | **str** | The name of the merchant that needs to be displayed on Gpay PopUp | 

## Example

```python
from hyperswitch.models.gpay_merchant_info import GpayMerchantInfo

# TODO update the JSON string below
json = "{}"
# create an instance of GpayMerchantInfo from a JSON string
gpay_merchant_info_instance = GpayMerchantInfo.from_json(json)
# print the JSON string representation of the object
print(GpayMerchantInfo.to_json())

# convert the object into a dict
gpay_merchant_info_dict = gpay_merchant_info_instance.to_dict()
# create an instance of GpayMerchantInfo from a dict
gpay_merchant_info_from_dict = GpayMerchantInfo.from_dict(gpay_merchant_info_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


