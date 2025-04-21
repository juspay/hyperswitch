from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.country_alpha_2 import CountryAlpha2
from ..types import UNSET, Unset

T = TypeVar("T", bound="GiropayBankRedirectAdditionalData")


@_attrs_define
class GiropayBankRedirectAdditionalData:
    """
    Attributes:
        bic (Union[None, Unset, str]): Masked bank account bic code
        iban (Union[None, Unset, str]): Partially masked international bank account number (iban) for SEPA
        country (Union[CountryAlpha2, None, Unset]):
    """

    bic: Union[None, Unset, str] = UNSET
    iban: Union[None, Unset, str] = UNSET
    country: Union[CountryAlpha2, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        bic: Union[None, Unset, str]
        if isinstance(self.bic, Unset):
            bic = UNSET
        else:
            bic = self.bic

        iban: Union[None, Unset, str]
        if isinstance(self.iban, Unset):
            iban = UNSET
        else:
            iban = self.iban

        country: Union[None, Unset, str]
        if isinstance(self.country, Unset):
            country = UNSET
        elif isinstance(self.country, CountryAlpha2):
            country = self.country.value
        else:
            country = self.country

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if bic is not UNSET:
            field_dict["bic"] = bic
        if iban is not UNSET:
            field_dict["iban"] = iban
        if country is not UNSET:
            field_dict["country"] = country

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_bic(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bic = _parse_bic(d.pop("bic", UNSET))

        def _parse_iban(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        iban = _parse_iban(d.pop("iban", UNSET))

        def _parse_country(data: object) -> Union[CountryAlpha2, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                country_type_1 = CountryAlpha2(data)

                return country_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CountryAlpha2, None, Unset], data)

        country = _parse_country(d.pop("country", UNSET))

        giropay_bank_redirect_additional_data = cls(
            bic=bic,
            iban=iban,
            country=country,
        )

        giropay_bank_redirect_additional_data.additional_properties = d
        return giropay_bank_redirect_additional_data

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
