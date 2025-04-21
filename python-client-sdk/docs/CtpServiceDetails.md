# CtpServiceDetails


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**merchant_transaction_id** | **str** | merchant transaction id | [optional] 
**correlation_id** | **str** | network transaction correlation id | [optional] 
**x_src_flow_id** | **str** | session transaction flow id | [optional] 
**provider** | [**CtpServiceProvider**](CtpServiceProvider.md) |  | [optional] 
**encypted_payload** | **str** | Encrypted payload | [optional] 

## Example

```python
from hyperswitch.models.ctp_service_details import CtpServiceDetails

# TODO update the JSON string below
json = "{}"
# create an instance of CtpServiceDetails from a JSON string
ctp_service_details_instance = CtpServiceDetails.from_json(json)
# print the JSON string representation of the object
print(CtpServiceDetails.to_json())

# convert the object into a dict
ctp_service_details_dict = ctp_service_details_instance.to_dict()
# create an instance of CtpServiceDetails from a dict
ctp_service_details_from_dict = CtpServiceDetails.from_dict(ctp_service_details_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


