# RequestSurchargeDetails

Details of surcharge applied on this payment, if applicable

## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**surcharge_amount** | **int** |  | 
**tax_amount** | **int** | This Unit struct represents MinorUnit in which core amount works | [optional] 

## Example

```python
from hyperswitch.models.request_surcharge_details import RequestSurchargeDetails

# TODO update the JSON string below
json = "{}"
# create an instance of RequestSurchargeDetails from a JSON string
request_surcharge_details_instance = RequestSurchargeDetails.from_json(json)
# print the JSON string representation of the object
print(RequestSurchargeDetails.to_json())

# convert the object into a dict
request_surcharge_details_dict = request_surcharge_details_instance.to_dict()
# create an instance of RequestSurchargeDetails from a dict
request_surcharge_details_from_dict = RequestSurchargeDetails.from_dict(request_surcharge_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


