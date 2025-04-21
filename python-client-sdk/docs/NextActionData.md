# NextActionData


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**redirect_to_url** | **str** |  | 
**type** | **str** |  | 
**bank_transfer_steps_and_charges_details** | [**BankTransferNextStepsData**](BankTransferNextStepsData.md) |  | 
**session_token** | [**SessionToken**](SessionToken.md) |  | [optional] 
**image_data_url** | **str** | Hyperswitch generated image data source url | 
**display_to_timestamp** | **int** |  | [optional] 
**qr_code_url** | **str** | The url for Qr code given by the connector | 
**display_text** | **str** |  | [optional] 
**border_color** | **str** |  | [optional] 
**qr_code_fetch_url** | **str** |  | 
**voucher_details** | **str** |  | 
**display_from_timestamp** | **int** |  | 
**three_ds_data** | [**ThreeDsData**](ThreeDsData.md) |  | 
**next_action_data** | [**SdkNextActionData**](SdkNextActionData.md) |  | 
**consent_data_required** | [**MobilePaymentConsent**](MobilePaymentConsent.md) |  | 
**iframe_data** | [**IframeData**](IframeData.md) |  | 

## Example

```python
from hyperswitch.models.next_action_data import NextActionData

# TODO update the JSON string below
json = "{}"
# create an instance of NextActionData from a JSON string
next_action_data_instance = NextActionData.from_json(json)
# print the JSON string representation of the object
print(NextActionData.to_json())

# convert the object into a dict
next_action_data_dict = next_action_data_instance.to_dict()
# create an instance of NextActionData from a dict
next_action_data_from_dict = NextActionData.from_dict(next_action_data_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


