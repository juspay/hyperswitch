# SurchargeDetailsResponse


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**surcharge** | [**SurchargeResponse**](SurchargeResponse.md) |  | 
**tax_on_surcharge** | [**SurchargePercentage**](SurchargePercentage.md) |  | [optional] 
**display_surcharge_amount** | **float** | surcharge amount for this payment | 
**display_tax_on_surcharge_amount** | **float** | tax on surcharge amount for this payment | 
**display_total_surcharge_amount** | **float** | sum of display_surcharge_amount and display_tax_on_surcharge_amount | 

## Example

```python
from hyperswitch.models.surcharge_details_response import SurchargeDetailsResponse

# TODO update the JSON string below
json = "{}"
# create an instance of SurchargeDetailsResponse from a JSON string
surcharge_details_response_instance = SurchargeDetailsResponse.from_json(json)
# print the JSON string representation of the object
print(SurchargeDetailsResponse.to_json())

# convert the object into a dict
surcharge_details_response_dict = surcharge_details_response_instance.to_dict()
# create an instance of SurchargeDetailsResponse from a dict
surcharge_details_response_from_dict = SurchargeDetailsResponse.from_dict(surcharge_details_response_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


