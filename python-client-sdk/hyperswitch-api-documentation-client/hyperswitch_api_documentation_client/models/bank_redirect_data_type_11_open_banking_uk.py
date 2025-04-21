from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.bank_names import BankNames
from ..models.country_alpha_2 import CountryAlpha2

T = TypeVar("T", bound="BankRedirectDataType11OpenBankingUk")


@_attrs_define
class BankRedirectDataType11OpenBankingUk:
    """
    Attributes:
        issuer (BankNames): Name of banks supported by Hyperswitch
        country (CountryAlpha2):
    """

    issuer: BankNames
    country: CountryAlpha2
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        issuer = self.issuer.value

        country = self.country.value

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "issuer": issuer,
                "country": country,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        issuer = BankNames(d.pop("issuer"))

        country = CountryAlpha2(d.pop("country"))

        bank_redirect_data_type_11_open_banking_uk = cls(
            issuer=issuer,
            country=country,
        )

        bank_redirect_data_type_11_open_banking_uk.additional_properties = d
        return bank_redirect_data_type_11_open_banking_uk

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
