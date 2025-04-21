# MultibancoBillingDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **str** |  | [optional] 

## Example

```python
from hyperswitch.models.multibanco_billing_details import MultibancoBillingDetails

# TODO update the JSON string below
json = "{}"
# create an instance of MultibancoBillingDetails from a JSON string
multibanco_billing_details_instance = MultibancoBillingDetails.from_json(json)
# print the JSON string representation of the object
print(MultibancoBillingDetails.to_json())

# convert the object into a dict
multibanco_billing_details_dict = multibanco_billing_details_instance.to_dict()
# create an instance of MultibancoBillingDetails from a dict
multibanco_billing_details_from_dict = MultibancoBillingDetails.from_dict(multibanco_billing_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


