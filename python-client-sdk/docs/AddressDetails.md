# AddressDetails

Address details

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**city** | **str** | The address city | [optional] 
**country** | [**CountryAlpha2**](CountryAlpha2.md) |  | [optional] 
**line1** | **str** | The first line of the address | [optional] 
**line2** | **str** | The second line of the address | [optional] 
**line3** | **str** | The third line of the address | [optional] 
**zip** | **str** | The zip/postal code for the address | [optional] 
**state** | **str** | The address state | [optional] 
**first_name** | **str** | The first name for the address | [optional] 
**last_name** | **str** | The last name for the address | [optional] 

## Example

```python
from hyperswitch.models.address_details import AddressDetails

# TODO update the JSON string below
json = "{}"
# create an instance of AddressDetails from a JSON string
address_details_instance = AddressDetails.from_json(json)
# print the JSON string representation of the object
print(AddressDetails.to_json())

# convert the object into a dict
address_details_dict = address_details_instance.to_dict()
# create an instance of AddressDetails from a dict
address_details_from_dict = AddressDetails.from_dict(address_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


