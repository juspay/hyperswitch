from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="BancontactBankRedirectAdditionalData")


@_attrs_define
class BancontactBankRedirectAdditionalData:
    """
    Attributes:
        last4 (Union[None, Unset, str]): Last 4 digits of the card number Example: 4242.
        card_exp_month (Union[None, Unset, str]): The card's expiry month Example: 12.
        card_exp_year (Union[None, Unset, str]): The card's expiry year Example: 24.
        card_holder_name (Union[None, Unset, str]): The card holder's name Example: John Test.
    """

    last4: Union[None, Unset, str] = UNSET
    card_exp_month: Union[None, Unset, str] = UNSET
    card_exp_year: Union[None, Unset, str] = UNSET
    card_holder_name: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        last4: Union[None, Unset, str]
        if isinstance(self.last4, Unset):
            last4 = UNSET
        else:
            last4 = self.last4

        card_exp_month: Union[None, Unset, str]
        if isinstance(self.card_exp_month, Unset):
            card_exp_month = UNSET
        else:
            card_exp_month = self.card_exp_month

        card_exp_year: Union[None, Unset, str]
        if isinstance(self.card_exp_year, Unset):
            card_exp_year = UNSET
        else:
            card_exp_year = self.card_exp_year

        card_holder_name: Union[None, Unset, str]
        if isinstance(self.card_holder_name, Unset):
            card_holder_name = UNSET
        else:
            card_holder_name = self.card_holder_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if last4 is not UNSET:
            field_dict["last4"] = last4
        if card_exp_month is not UNSET:
            field_dict["card_exp_month"] = card_exp_month
        if card_exp_year is not UNSET:
            field_dict["card_exp_year"] = card_exp_year
        if card_holder_name is not UNSET:
            field_dict["card_holder_name"] = card_holder_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_last4(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        last4 = _parse_last4(d.pop("last4", UNSET))

        def _parse_card_exp_month(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_exp_month = _parse_card_exp_month(d.pop("card_exp_month", UNSET))

        def _parse_card_exp_year(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_exp_year = _parse_card_exp_year(d.pop("card_exp_year", UNSET))

        def _parse_card_holder_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_holder_name = _parse_card_holder_name(d.pop("card_holder_name", UNSET))

        bancontact_bank_redirect_additional_data = cls(
            last4=last4,
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
        )

        bancontact_bank_redirect_additional_data.additional_properties = d
        return bancontact_bank_redirect_additional_data

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
