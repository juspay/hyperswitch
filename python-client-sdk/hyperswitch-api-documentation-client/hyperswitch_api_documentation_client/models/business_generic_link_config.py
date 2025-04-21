from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BusinessGenericLinkConfig")


@_attrs_define
class BusinessGenericLinkConfig:
    """
    Attributes:
        allowed_domains (list[str]): A list of allowed domains (glob patterns) where this link can be embedded / opened
            from
        logo (Union[None, Unset, str]): Merchant's display logo Example: https://hyperswitch.io/favicon.ico.
        merchant_name (Union[None, Unset, str]): Custom merchant name for the link Example: Hyperswitch.
        theme (Union[None, Unset, str]): Primary color to be used in the form represented in hex format Example:
            #4285F4.
        domain_name (Union[None, Unset, str]): Custom domain name to be used for hosting the link
    """

    allowed_domains: list[str]
    logo: Union[None, Unset, str] = UNSET
    merchant_name: Union[None, Unset, str] = UNSET
    theme: Union[None, Unset, str] = UNSET
    domain_name: Union[None, Unset, str] = UNSET
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

        business_generic_link_config = cls(
            allowed_domains=allowed_domains,
            logo=logo,
            merchant_name=merchant_name,
            theme=theme,
            domain_name=domain_name,
        )

        business_generic_link_config.additional_properties = d
        return business_generic_link_config

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
