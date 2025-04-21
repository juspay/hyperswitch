# MerchantDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**primary_contact_person** | **str** | The merchant&#39;s primary contact name | [optional] 
**primary_phone** | **str** | The merchant&#39;s primary phone number | [optional] 
**primary_email** | **str** | The merchant&#39;s primary email address | [optional] 
**secondary_contact_person** | **str** | The merchant&#39;s secondary contact name | [optional] 
**secondary_phone** | **str** | The merchant&#39;s secondary phone number | [optional] 
**secondary_email** | **str** | The merchant&#39;s secondary email address | [optional] 
**website** | **str** | The business website of the merchant | [optional] 
**about_business** | **str** | A brief description about merchant&#39;s business | [optional] 
**address** | [**AddressDetails**](AddressDetails.md) |  | [optional] 

## Example

```python
from hyperswitch.models.merchant_details import MerchantDetails

# TODO update the JSON string below
json = "{}"
# create an instance of MerchantDetails from a JSON string
merchant_details_instance = MerchantDetails.from_json(json)
# print the JSON string representation of the object
print(MerchantDetails.to_json())

# convert the object into a dict
merchant_details_dict = merchant_details_instance.to_dict()
# create an instance of MerchantDetails from a dict
merchant_details_from_dict = MerchantDetails.from_dict(merchant_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


