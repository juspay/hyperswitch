from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.ui_widget_form_layout import UIWidgetFormLayout
from ..types import UNSET, Unset

T = TypeVar("T", bound="BusinessPayoutLinkConfig")


@_attrs_define
class BusinessPayoutLinkConfig:
    """
    Attributes:
        allowed_domains (list[str]): A list of allowed domains (glob patterns) where this link can be embedded / opened
            from
        logo (Union[None, Unset, str]): Merchant's display logo Example: https://hyperswitch.io/favicon.ico.
        merchant_name (Union[None, Unset, str]): Custom merchant name for the link Example: Hyperswitch.
        theme (Union[None, Unset, str]): Primary color to be used in the form represented in hex format Example:
            #4285F4.
        domain_name (Union[None, Unset, str]): Custom domain name to be used for hosting the link
        form_layout (Union[None, UIWidgetFormLayout, Unset]):
        payout_test_mode (Union[None, Unset, bool]): Allows for removing any validations / pre-requisites which are
            necessary in a production environment Default: False.
    """

    allowed_domains: list[str]
    logo: Union[None, Unset, str] = UNSET
    merchant_name: Union[None, Unset, str] = UNSET
    theme: Union[None, Unset, str] = UNSET
    domain_name: Union[None, Unset, str] = UNSET
    form_layout: Union[None, UIWidgetFormLayout, Unset] = UNSET
    payout_test_mode: Union[None, Unset, bool] = False
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        allowed_domains = self.allowed_domains

        logo: Union[None, Unset, str]
        if isinstance(self.logo, Unset):
            logo = UNSET
        else:
            logo = self.logo

        merchant_name: Union[None, Unset, str]
        if isinstance(self.merchant_name, Unset):
            merchant_name = UNSET
        else:
            merchant_name = self.merchant_name

        theme: Union[None, Unset, str]
        if isinstance(self.theme, Unset):
            theme = UNSET
        else:
            theme = self.theme

        domain_name: Union[None, Unset, str]
        if isinstance(self.domain_name, Unset):
            domain_name = UNSET
        else:
            domain_name = self.domain_name

        form_layout: Union[None, Unset, str]
        if isinstance(self.form_layout, Unset):
            form_layout = UNSET
        elif isinstance(self.form_layout, UIWidgetFormLayout):
            form_layout = self.form_layout.value
        else:
            form_layout = self.form_layout

        payout_test_mode: Union[None, Unset, bool]
        if isinstance(self.payout_test_mode, Unset):
            payout_test_mode = UNSET
        else:
            payout_test_mode = self.payout_test_mode

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "allowed_domains": allowed_domains,
            }
        )
        if logo is not UNSET:
            field_dict["logo"] = logo
        if merchant_name is not UNSET:
            field_dict["merchant_name"] = merchant_name
        if theme is not UNSET:
            field_dict["theme"] = theme
        if domain_name is not UNSET:
            field_dict["domain_name"] = domain_name
        if form_layout is not UNSET:
            field_dict["form_layout"] = form_layout
        if payout_test_mode is not UNSET:
            field_dict["payout_test_mode"] = payout_test_mode

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        allowed_domains = cast(list[str], d.pop("allowed_domains"))

        def _parse_logo(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        logo = _parse_logo(d.pop("logo", UNSET))

        def _parse_merchant_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_name = _parse_merchant_name(d.pop("merchant_name", UNSET))

        def _parse_theme(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        theme = _parse_theme(d.pop("theme", UNSET))

        def _parse_domain_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        domain_name = _parse_domain_name(d.pop("domain_name", UNSET))

        def _parse_form_layout(data: object) -> Union[None, UIWidgetFormLayout, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                form_layout_type_1 = UIWidgetFormLayout(data)

                return form_layout_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, UIWidgetFormLayout, Unset], data)

        form_layout = _parse_form_layout(d.pop("form_layout", UNSET))

        def _parse_payout_test_mode(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        payout_test_mode = _parse_payout_test_mode(d.pop("payout_test_mode", UNSET))

        business_payout_link_config = cls(
            allowed_domains=allowed_domains,
            logo=logo,
            merchant_name=merchant_name,
            theme=theme,
            domain_name=domain_name,
            form_layout=form_layout,
            payout_test_mode=payout_test_mode,
        )

        business_payout_link_config.additional_properties = d
        return business_payout_link_config

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
