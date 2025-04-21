from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.payment_link_details_layout import PaymentLinkDetailsLayout
from ..models.payment_link_sdk_label_type import PaymentLinkSdkLabelType
from ..models.payment_link_show_sdk_terms import PaymentLinkShowSdkTerms
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.payment_link_background_image_config import PaymentLinkBackgroundImageConfig
    from ..models.payment_link_config_payment_link_ui_rules_type_0 import PaymentLinkConfigPaymentLinkUiRulesType0
    from ..models.payment_link_config_sdk_ui_rules_type_0 import PaymentLinkConfigSdkUiRulesType0
    from ..models.payment_link_transaction_details import PaymentLinkTransactionDetails


T = TypeVar("T", bound="PaymentLinkConfig")


@_attrs_define
class PaymentLinkConfig:
    """
    Attributes:
        theme (str): custom theme for the payment link
        logo (str): merchant display logo
        seller_name (str): Custom merchant name for payment link
        sdk_layout (str): Custom layout for sdk
        display_sdk_only (bool): Display only the sdk for payment link
        enabled_saved_payment_method (bool): Enable saved payment method option for payment link
        hide_card_nickname_field (bool): Hide card nickname field option for payment link
        show_card_form_by_default (bool): Show card form by default for payment link
        enable_button_only_on_form_ready (bool): Flag to enable the button only when the payment form is ready for
            submission
        allowed_domains (Union[None, Unset, list[str]]): A list of allowed domains (glob patterns) where this link can
            be embedded / opened from
        transaction_details (Union[None, Unset, list['PaymentLinkTransactionDetails']]): Dynamic details related to
            merchant to be rendered in payment link
        background_image (Union['PaymentLinkBackgroundImageConfig', None, Unset]):
        details_layout (Union[None, PaymentLinkDetailsLayout, Unset]):
        branding_visibility (Union[None, Unset, bool]): Toggle for HyperSwitch branding visibility
        payment_button_text (Union[None, Unset, str]): Text for payment link's handle confirm button
        custom_message_for_card_terms (Union[None, Unset, str]): Text for customizing message for card terms
        payment_button_colour (Union[None, Unset, str]): Custom background colour for payment link's handle confirm
            button
        skip_status_screen (Union[None, Unset, bool]): Skip the status screen after payment completion
        payment_button_text_colour (Union[None, Unset, str]): Custom text colour for payment link's handle confirm
            button
        background_colour (Union[None, Unset, str]): Custom background colour for the payment link
        sdk_ui_rules (Union['PaymentLinkConfigSdkUiRulesType0', None, Unset]): SDK configuration rules
        payment_link_ui_rules (Union['PaymentLinkConfigPaymentLinkUiRulesType0', None, Unset]): Payment link
            configuration rules
        payment_form_header_text (Union[None, Unset, str]): Optional header for the SDK's payment form
        payment_form_label_type (Union[None, PaymentLinkSdkLabelType, Unset]):
        show_card_terms (Union[None, PaymentLinkShowSdkTerms, Unset]):
    """

    theme: str
    logo: str
    seller_name: str
    sdk_layout: str
    display_sdk_only: bool
    enabled_saved_payment_method: bool
    hide_card_nickname_field: bool
    show_card_form_by_default: bool
    enable_button_only_on_form_ready: bool
    allowed_domains: Union[None, Unset, list[str]] = UNSET
    transaction_details: Union[None, Unset, list["PaymentLinkTransactionDetails"]] = UNSET
    background_image: Union["PaymentLinkBackgroundImageConfig", None, Unset] = UNSET
    details_layout: Union[None, PaymentLinkDetailsLayout, Unset] = UNSET
    branding_visibility: Union[None, Unset, bool] = UNSET
    payment_button_text: Union[None, Unset, str] = UNSET
    custom_message_for_card_terms: Union[None, Unset, str] = UNSET
    payment_button_colour: Union[None, Unset, str] = UNSET
    skip_status_screen: Union[None, Unset, bool] = UNSET
    payment_button_text_colour: Union[None, Unset, str] = UNSET
    background_colour: Union[None, Unset, str] = UNSET
    sdk_ui_rules: Union["PaymentLinkConfigSdkUiRulesType0", None, Unset] = UNSET
    payment_link_ui_rules: Union["PaymentLinkConfigPaymentLinkUiRulesType0", None, Unset] = UNSET
    payment_form_header_text: Union[None, Unset, str] = UNSET
    payment_form_label_type: Union[None, PaymentLinkSdkLabelType, Unset] = UNSET
    show_card_terms: Union[None, PaymentLinkShowSdkTerms, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.payment_link_background_image_config import PaymentLinkBackgroundImageConfig
        from ..models.payment_link_config_payment_link_ui_rules_type_0 import PaymentLinkConfigPaymentLinkUiRulesType0
        from ..models.payment_link_config_sdk_ui_rules_type_0 import PaymentLinkConfigSdkUiRulesType0

        theme = self.theme

        logo = self.logo

        seller_name = self.seller_name

        sdk_layout = self.sdk_layout

        display_sdk_only = self.display_sdk_only

        enabled_saved_payment_method = self.enabled_saved_payment_method

        hide_card_nickname_field = self.hide_card_nickname_field

        show_card_form_by_default = self.show_card_form_by_default

        enable_button_only_on_form_ready = self.enable_button_only_on_form_ready

        allowed_domains: Union[None, Unset, list[str]]
        if isinstance(self.allowed_domains, Unset):
            allowed_domains = UNSET
        elif isinstance(self.allowed_domains, list):
            allowed_domains = self.allowed_domains

        else:
            allowed_domains = self.allowed_domains

        transaction_details: Union[None, Unset, list[dict[str, Any]]]
        if isinstance(self.transaction_details, Unset):
            transaction_details = UNSET
        elif isinstance(self.transaction_details, list):
            transaction_details = []
            for transaction_details_type_0_item_data in self.transaction_details:
                transaction_details_type_0_item = transaction_details_type_0_item_data.to_dict()
                transaction_details.append(transaction_details_type_0_item)

        else:
            transaction_details = self.transaction_details

        background_image: Union[None, Unset, dict[str, Any]]
        if isinstance(self.background_image, Unset):
            background_image = UNSET
        elif isinstance(self.background_image, PaymentLinkBackgroundImageConfig):
            background_image = self.background_image.to_dict()
        else:
            background_image = self.background_image

        details_layout: Union[None, Unset, str]
        if isinstance(self.details_layout, Unset):
            details_layout = UNSET
        elif isinstance(self.details_layout, PaymentLinkDetailsLayout):
            details_layout = self.details_layout.value
        else:
            details_layout = self.details_layout

        branding_visibility: Union[None, Unset, bool]
        if isinstance(self.branding_visibility, Unset):
            branding_visibility = UNSET
        else:
            branding_visibility = self.branding_visibility

        payment_button_text: Union[None, Unset, str]
        if isinstance(self.payment_button_text, Unset):
            payment_button_text = UNSET
        else:
            payment_button_text = self.payment_button_text

        custom_message_for_card_terms: Union[None, Unset, str]
        if isinstance(self.custom_message_for_card_terms, Unset):
            custom_message_for_card_terms = UNSET
        else:
            custom_message_for_card_terms = self.custom_message_for_card_terms

        payment_button_colour: Union[None, Unset, str]
        if isinstance(self.payment_button_colour, Unset):
            payment_button_colour = UNSET
        else:
            payment_button_colour = self.payment_button_colour

        skip_status_screen: Union[None, Unset, bool]
        if isinstance(self.skip_status_screen, Unset):
            skip_status_screen = UNSET
        else:
            skip_status_screen = self.skip_status_screen

        payment_button_text_colour: Union[None, Unset, str]
        if isinstance(self.payment_button_text_colour, Unset):
            payment_button_text_colour = UNSET
        else:
            payment_button_text_colour = self.payment_button_text_colour

        background_colour: Union[None, Unset, str]
        if isinstance(self.background_colour, Unset):
            background_colour = UNSET
        else:
            background_colour = self.background_colour

        sdk_ui_rules: Union[None, Unset, dict[str, Any]]
        if isinstance(self.sdk_ui_rules, Unset):
            sdk_ui_rules = UNSET
        elif isinstance(self.sdk_ui_rules, PaymentLinkConfigSdkUiRulesType0):
            sdk_ui_rules = self.sdk_ui_rules.to_dict()
        else:
            sdk_ui_rules = self.sdk_ui_rules

        payment_link_ui_rules: Union[None, Unset, dict[str, Any]]
        if isinstance(self.payment_link_ui_rules, Unset):
            payment_link_ui_rules = UNSET
        elif isinstance(self.payment_link_ui_rules, PaymentLinkConfigPaymentLinkUiRulesType0):
            payment_link_ui_rules = self.payment_link_ui_rules.to_dict()
        else:
            payment_link_ui_rules = self.payment_link_ui_rules

        payment_form_header_text: Union[None, Unset, str]
        if isinstance(self.payment_form_header_text, Unset):
            payment_form_header_text = UNSET
        else:
            payment_form_header_text = self.payment_form_header_text

        payment_form_label_type: Union[None, Unset, str]
        if isinstance(self.payment_form_label_type, Unset):
            payment_form_label_type = UNSET
        elif isinstance(self.payment_form_label_type, PaymentLinkSdkLabelType):
            payment_form_label_type = self.payment_form_label_type.value
        else:
            payment_form_label_type = self.payment_form_label_type

        show_card_terms: Union[None, Unset, str]
        if isinstance(self.show_card_terms, Unset):
            show_card_terms = UNSET
        elif isinstance(self.show_card_terms, PaymentLinkShowSdkTerms):
            show_card_terms = self.show_card_terms.value
        else:
            show_card_terms = self.show_card_terms

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "theme": theme,
                "logo": logo,
                "seller_name": seller_name,
                "sdk_layout": sdk_layout,
                "display_sdk_only": display_sdk_only,
                "enabled_saved_payment_method": enabled_saved_payment_method,
                "hide_card_nickname_field": hide_card_nickname_field,
                "show_card_form_by_default": show_card_form_by_default,
                "enable_button_only_on_form_ready": enable_button_only_on_form_ready,
            }
        )
        if allowed_domains is not UNSET:
            field_dict["allowed_domains"] = allowed_domains
        if transaction_details is not UNSET:
            field_dict["transaction_details"] = transaction_details
        if background_image is not UNSET:
            field_dict["background_image"] = background_image
        if details_layout is not UNSET:
            field_dict["details_layout"] = details_layout
        if branding_visibility is not UNSET:
            field_dict["branding_visibility"] = branding_visibility
        if payment_button_text is not UNSET:
            field_dict["payment_button_text"] = payment_button_text
        if custom_message_for_card_terms is not UNSET:
            field_dict["custom_message_for_card_terms"] = custom_message_for_card_terms
        if payment_button_colour is not UNSET:
            field_dict["payment_button_colour"] = payment_button_colour
        if skip_status_screen is not UNSET:
            field_dict["skip_status_screen"] = skip_status_screen
        if payment_button_text_colour is not UNSET:
            field_dict["payment_button_text_colour"] = payment_button_text_colour
        if background_colour is not UNSET:
            field_dict["background_colour"] = background_colour
        if sdk_ui_rules is not UNSET:
            field_dict["sdk_ui_rules"] = sdk_ui_rules
        if payment_link_ui_rules is not UNSET:
            field_dict["payment_link_ui_rules"] = payment_link_ui_rules
        if payment_form_header_text is not UNSET:
            field_dict["payment_form_header_text"] = payment_form_header_text
        if payment_form_label_type is not UNSET:
            field_dict["payment_form_label_type"] = payment_form_label_type
        if show_card_terms is not UNSET:
            field_dict["show_card_terms"] = show_card_terms

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.payment_link_background_image_config import PaymentLinkBackgroundImageConfig
        from ..models.payment_link_config_payment_link_ui_rules_type_0 import PaymentLinkConfigPaymentLinkUiRulesType0
        from ..models.payment_link_config_sdk_ui_rules_type_0 import PaymentLinkConfigSdkUiRulesType0
        from ..models.payment_link_transaction_details import PaymentLinkTransactionDetails

        d = dict(src_dict)
        theme = d.pop("theme")

        logo = d.pop("logo")

        seller_name = d.pop("seller_name")

        sdk_layout = d.pop("sdk_layout")

        display_sdk_only = d.pop("display_sdk_only")

        enabled_saved_payment_method = d.pop("enabled_saved_payment_method")

        hide_card_nickname_field = d.pop("hide_card_nickname_field")

        show_card_form_by_default = d.pop("show_card_form_by_default")

        enable_button_only_on_form_ready = d.pop("enable_button_only_on_form_ready")

        def _parse_allowed_domains(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                allowed_domains_type_0 = cast(list[str], data)

                return allowed_domains_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        allowed_domains = _parse_allowed_domains(d.pop("allowed_domains", UNSET))

        def _parse_transaction_details(data: object) -> Union[None, Unset, list["PaymentLinkTransactionDetails"]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                transaction_details_type_0 = []
                _transaction_details_type_0 = data
                for transaction_details_type_0_item_data in _transaction_details_type_0:
                    transaction_details_type_0_item = PaymentLinkTransactionDetails.from_dict(
                        transaction_details_type_0_item_data
                    )

                    transaction_details_type_0.append(transaction_details_type_0_item)

                return transaction_details_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list["PaymentLinkTransactionDetails"]], data)

        transaction_details = _parse_transaction_details(d.pop("transaction_details", UNSET))

        def _parse_background_image(data: object) -> Union["PaymentLinkBackgroundImageConfig", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                background_image_type_1 = PaymentLinkBackgroundImageConfig.from_dict(data)

                return background_image_type_1
            except:  # noqa: E722
                pass
            return cast(Union["PaymentLinkBackgroundImageConfig", None, Unset], data)

        background_image = _parse_background_image(d.pop("background_image", UNSET))

        def _parse_details_layout(data: object) -> Union[None, PaymentLinkDetailsLayout, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                details_layout_type_1 = PaymentLinkDetailsLayout(data)

                return details_layout_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentLinkDetailsLayout, Unset], data)

        details_layout = _parse_details_layout(d.pop("details_layout", UNSET))

        def _parse_branding_visibility(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        branding_visibility = _parse_branding_visibility(d.pop("branding_visibility", UNSET))

        def _parse_payment_button_text(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_button_text = _parse_payment_button_text(d.pop("payment_button_text", UNSET))

        def _parse_custom_message_for_card_terms(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        custom_message_for_card_terms = _parse_custom_message_for_card_terms(
            d.pop("custom_message_for_card_terms", UNSET)
        )

        def _parse_payment_button_colour(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_button_colour = _parse_payment_button_colour(d.pop("payment_button_colour", UNSET))

        def _parse_skip_status_screen(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        skip_status_screen = _parse_skip_status_screen(d.pop("skip_status_screen", UNSET))

        def _parse_payment_button_text_colour(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_button_text_colour = _parse_payment_button_text_colour(d.pop("payment_button_text_colour", UNSET))

        def _parse_background_colour(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        background_colour = _parse_background_colour(d.pop("background_colour", UNSET))

        def _parse_sdk_ui_rules(data: object) -> Union["PaymentLinkConfigSdkUiRulesType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                sdk_ui_rules_type_0 = PaymentLinkConfigSdkUiRulesType0.from_dict(data)

                return sdk_ui_rules_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentLinkConfigSdkUiRulesType0", None, Unset], data)

        sdk_ui_rules = _parse_sdk_ui_rules(d.pop("sdk_ui_rules", UNSET))

        def _parse_payment_link_ui_rules(
            data: object,
        ) -> Union["PaymentLinkConfigPaymentLinkUiRulesType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                payment_link_ui_rules_type_0 = PaymentLinkConfigPaymentLinkUiRulesType0.from_dict(data)

                return payment_link_ui_rules_type_0
            except:  # noqa: E722
                pass
            return cast(Union["PaymentLinkConfigPaymentLinkUiRulesType0", None, Unset], data)

        payment_link_ui_rules = _parse_payment_link_ui_rules(d.pop("payment_link_ui_rules", UNSET))

        def _parse_payment_form_header_text(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_form_header_text = _parse_payment_form_header_text(d.pop("payment_form_header_text", UNSET))

        def _parse_payment_form_label_type(data: object) -> Union[None, PaymentLinkSdkLabelType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_form_label_type_type_1 = PaymentLinkSdkLabelType(data)

                return payment_form_label_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentLinkSdkLabelType, Unset], data)

        payment_form_label_type = _parse_payment_form_label_type(d.pop("payment_form_label_type", UNSET))

        def _parse_show_card_terms(data: object) -> Union[None, PaymentLinkShowSdkTerms, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                show_card_terms_type_1 = PaymentLinkShowSdkTerms(data)

                return show_card_terms_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentLinkShowSdkTerms, Unset], data)

        show_card_terms = _parse_show_card_terms(d.pop("show_card_terms", UNSET))

        payment_link_config = cls(
            theme=theme,
            logo=logo,
            seller_name=seller_name,
            sdk_layout=sdk_layout,
            display_sdk_only=display_sdk_only,
            enabled_saved_payment_method=enabled_saved_payment_method,
            hide_card_nickname_field=hide_card_nickname_field,
            show_card_form_by_default=show_card_form_by_default,
            enable_button_only_on_form_ready=enable_button_only_on_form_ready,
            allowed_domains=allowed_domains,
            transaction_details=transaction_details,
            background_image=background_image,
            details_layout=details_layout,
            branding_visibility=branding_visibility,
            payment_button_text=payment_button_text,
            custom_message_for_card_terms=custom_message_for_card_terms,
            payment_button_colour=payment_button_colour,
            skip_status_screen=skip_status_screen,
            payment_button_text_colour=payment_button_text_colour,
            background_colour=background_colour,
            sdk_ui_rules=sdk_ui_rules,
            payment_link_ui_rules=payment_link_ui_rules,
            payment_form_header_text=payment_form_header_text,
            payment_form_label_type=payment_form_label_type,
            show_card_terms=show_card_terms,
        )

        payment_link_config.additional_properties = d
        return payment_link_config

    @property
    def additional_keys(self) -> list[str]:
        return list(self.additional_properties.keys())

    def __getitem__(self, key: str) -> Any:
        return self.additional_properties[key]

    def __setitem__(self, key: str, value: Any) -> None:
        self.additional_properties[key] = value

    def __delitem__(self, key: str) -> None:
        del self.additional_properties[key]

    def __contains__(self, key: str) -> bool:
        return key in self.additional_properties
