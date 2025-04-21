# SepaAndBacsBillingDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**email** | **str** | The Email ID for SEPA and BACS billing | [optional] 
**name** | **str** | The billing name for SEPA and BACS billing | [optional] 

## Example

```python
from hyperswitch.models.sepa_and_bacs_billing_details import SepaAndBacsBillingDetails

# TODO update the JSON string below
json = "{}"
# create an instance of SepaAndBacsBillingDetails from a JSON string
sepa_and_bacs_billing_details_instance = SepaAndBacsBillingDetails.from_json(json)
# print the JSON string representation of the object
print(SepaAndBacsBillingDetails.to_json())

# convert the object into a dict
sepa_and_bacs_billing_details_dict = sepa_and_bacs_billing_details_instance.to_dict()
# create an instance of SepaAndBacsBillingDetails from a dict
sepa_and_bacs_billing_details_from_dict = SepaAndBacsBillingDetails.from_dict(sepa_and_bacs_billing_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


