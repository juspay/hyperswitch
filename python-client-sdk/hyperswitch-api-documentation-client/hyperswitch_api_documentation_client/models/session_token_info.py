from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.applepay_initiative import ApplepayInitiative
from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

T = TypeVar("T", bound="SessionTokenInfo")


@_attrs_define
class SessionTokenInfo:
    """
    Attributes:
        certificate (str):
        certificate_keys (str):
        merchant_identifier (str):
        display_name (str):
        initiative (ApplepayInitiative):
        initiative_context (Union[None, Unset, str]):
        merchant_business_country (Union[CountryAlpha2, None, Unset]):
    """

    certificate: str
    certificate_keys: str
    merchant_identifier: str
    display_name: str
    initiative: ApplepayInitiative
    initiative_context: Union[None, Unset, str] = UNSET
    merchant_business_country: Union[CountryAlpha2, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        certificate = self.certificate

        certificate_keys = self.certificate_keys

        merchant_identifier = self.merchant_identifier

        display_name = self.display_name

        initiative = self.initiative.value

        initiative_context: Union[None, Unset, str]
        if isinstance(self.initiative_context, Unset):
            initiative_context = UNSET
        else:
            initiative_context = self.initiative_context

        merchant_business_country: Union[None, Unset, str]
        if isinstance(self.merchant_business_country, Unset):
            merchant_business_country = UNSET
        elif isinstance(self.merchant_business_country, CountryAlpha2):
            merchant_business_country = self.merchant_business_country.value
        else:
            merchant_business_country = self.merchant_business_country

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "certificate": certificate,
                "certificate_keys": certificate_keys,
                "merchant_identifier": merchant_identifier,
                "display_name": display_name,
                "initiative": initiative,
            }
        )
        if initiative_context is not UNSET:
            field_dict["initiative_context"] = initiative_context
        if merchant_business_country is not UNSET:
            field_dict["merchant_business_country"] = merchant_business_country

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        certificate = d.pop("certificate")

        certificate_keys = d.pop("certificate_keys")

        merchant_identifier = d.pop("merchant_identifier")

        display_name = d.pop("display_name")

        initiative = ApplepayInitiative(d.pop("initiative"))

        def _parse_initiative_context(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        initiative_context = _parse_initiative_context(d.pop("initiative_context", UNSET))

        def _parse_merchant_business_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                merchant_business_country_type_1 = CountryAlpha2(data)

                return merchant_business_country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        merchant_business_country = _parse_merchant_business_country(d.pop("merchant_business_country", UNSET))

        session_token_info = cls(
            certificate=certificate,
            certificate_keys=certificate_keys,
            merchant_identifier=merchant_identifier,
            display_name=display_name,
            initiative=initiative,
            initiative_context=initiative_context,
            merchant_business_country=merchant_business_country,
        )

        session_token_info.additional_properties = d
        return session_token_info

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
