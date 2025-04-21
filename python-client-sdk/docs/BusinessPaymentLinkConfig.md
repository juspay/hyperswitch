# BusinessPaymentLinkConfig


## Properties

Name | Type | Description | Notes
------------ | ------------- | ------------- | -------------
**theme** | **str** | custom theme for the payment link | [optional] 
**logo** | **str** | merchant display logo | [optional] 
**seller_name** | **str** | Custom merchant name for payment link | [optional] 
**sdk_layout** | **str** | Custom layout for sdk | [optional] 
**display_sdk_only** | **bool** | Display only the sdk for payment link | [optional] [default to False]
**enabled_saved_payment_method** | **bool** | Enable saved payment method option for payment link | [optional] [default to False]
**hide_card_nickname_field** | **bool** | Hide card nickname field option for payment link | [optional] [default to False]
**show_card_form_by_default** | **bool** | Show card form by default for payment link | [optional] [default to True]
**transaction_details** | [**List[PaymentLinkTransactionDetails]**](PaymentLinkTransactionDetails.md) | Dynamic details related to merchant to be rendered in payment link | [optional] 
**background_image** | [**PaymentLinkBackgroundImageConfig**](PaymentLinkBackgroundImageConfig.md) |  | [optional] 
**details_layout** | [**PaymentLinkDetailsLayout**](PaymentLinkDetailsLayout.md) |  | [optional] 
**payment_button_text** | **str** | Text for payment link&#39;s handle confirm button | [optional] 
**custom_message_for_card_terms** | **str** | Text for customizing message for card terms | [optional] 
**payment_button_colour** | **str** | Custom background colour for payment link&#39;s handle confirm button | [optional] 
**skip_status_screen** | **bool** | Skip the status screen after payment completion | [optional] 
**payment_button_text_colour** | **str** | Custom text colour for payment link&#39;s handle confirm button | [optional] 
**background_colour** | **str** | Custom background colour for the payment link | [optional] 
**sdk_ui_rules** | **Dict[str, Dict[str, str]]** | SDK configuration rules | [optional] 
**payment_link_ui_rules** | **Dict[str, Dict[str, str]]** | Payment link configuration rules | [optional] 
**enable_button_only_on_form_ready** | **bool** | Flag to enable the button only when the payment form is ready for submission | [optional] 
**payment_form_header_text** | **str** | Optional header for the SDK&#39;s payment form | [optional] 
**payment_form_label_type** | [**PaymentLinkSdkLabelType**](PaymentLinkSdkLabelType.md) |  | [optional] 
**show_card_terms** | [**PaymentLinkShowSdkTerms**](PaymentLinkShowSdkTerms.md) |  | [optional] 
**domain_name** | **str** | Custom domain name to be used for hosting the link in your own domain | [optional] 
**business_specific_configs** | [**Dict[str, PaymentLinkConfigRequest]**](PaymentLinkConfigRequest.md) | list of configs for multi theme setup | [optional] 
**allowed_domains** | **List[str]** | A list of allowed domains (glob patterns) where this link can be embedded / opened from | [optional] 
**branding_visibility** | **bool** | Toggle for HyperSwitch branding visibility | [optional] 

## Example

```python
from hyperswitch.models.business_payment_link_config import BusinessPaymentLinkConfig

# TODO update the JSON string below
json = "{}"
# create an instance of BusinessPaymentLinkConfig from a JSON string
business_payment_link_config_instance = BusinessPaymentLinkConfig.from_json(json)
# print the JSON string representation of the object
print(BusinessPaymentLinkConfig.to_json())

# convert the object into a dict
business_payment_link_config_dict = business_payment_link_config_instance.to_dict()
# create an instance of BusinessPaymentLinkConfig from a dict
business_payment_link_config_from_dict = BusinessPaymentLinkConfig.from_dict(business_payment_link_config_dict)
```
[[Back to Model list]](../README.md#documentation-for-models) [[Back to API list]](../README.md#documentation-for-api-endpoints) [[Back to README]](../README.md)


