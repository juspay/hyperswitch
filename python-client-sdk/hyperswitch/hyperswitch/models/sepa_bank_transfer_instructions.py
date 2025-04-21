from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="SepaBankTransferInstructions")


@_attrs_define
class SepaBankTransferInstructions:
    """
    Attributes:
        account_holder_name (str):  Example: Jane Doe.
        bic (str):  Example: 9123456789.
        country (str):
        iban (str):  Example: 123456789.
        reference (str):  Example: U2PVVSEV4V9Y.
    """

    account_holder_name: str
    bic: str
    country: str
    iban: str
    reference: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        account_holder_name = self.account_holder_name

        bic = self.bic

        country = self.country

        iban = self.iban

        reference = self.reference

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "account_holder_name": account_holder_name,
                "bic": bic,
                "country": country,
                "iban": iban,
                "reference": reference,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        account_holder_name = d.pop("account_holder_name")

        bic = d.pop("bic")

        country = d.pop("country")

        iban = d.pop("iban")

        reference = d.pop("reference")

        sepa_bank_transfer_instructions = cls(
            account_holder_name=account_holder_name,
            bic=bic,
            country=country,
            iban=iban,
            reference=reference,
        )

        sepa_bank_transfer_instructions.additional_properties = d
        return sepa_bank_transfer_instructions

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
