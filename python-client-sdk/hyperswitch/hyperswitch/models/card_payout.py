from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="CardPayout")


@_attrs_define
class CardPayout:
    """
    Attributes:
        card_number (str): The card number Example: 4242424242424242.
        expiry_month (str): The card's expiry month
        expiry_year (str): The card's expiry year
        card_holder_name (str): The card holder's name Example: John Doe.
    """

    card_number: str
    expiry_month: str
    expiry_year: str
    card_holder_name: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_number = self.card_number

        expiry_month = self.expiry_month

        expiry_year = self.expiry_year

        card_holder_name = self.card_holder_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_number": card_number,
                "expiry_month": expiry_month,
                "expiry_year": expiry_year,
                "card_holder_name": card_holder_name,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_number = d.pop("card_number")

        expiry_month = d.pop("expiry_month")

        expiry_year = d.pop("expiry_year")

        card_holder_name = d.pop("card_holder_name")

        card_payout = cls(
            card_number=card_number,
            expiry_month=expiry_month,
            expiry_year=expiry_year,
            card_holder_name=card_holder_name,
        )

        card_payout.additional_properties = d
        return card_payout

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
