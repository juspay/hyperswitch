from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

T = TypeVar("T", bound="CardDetailUpdate")


@_attrs_define
class CardDetailUpdate:
    """
    Attributes:
        card_exp_month (str): Card Expiry Month Example: 10.
        card_exp_year (str): Card Expiry Year Example: 25.
        card_holder_name (str): Card Holder Name Example: John Doe.
        nick_name (Union[None, Unset, str]): Card Holder's Nick Name Example: John Doe.
    """

    card_exp_month: str
    card_exp_year: str
    card_holder_name: str
    nick_name: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        card_exp_month = self.card_exp_month

        card_exp_year = self.card_exp_year

        card_holder_name = self.card_holder_name

        nick_name: Union[None, Unset, str]
        if isinstance(self.nick_name, Unset):
            nick_name = UNSET
        else:
            nick_name = self.nick_name

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "card_exp_month": card_exp_month,
                "card_exp_year": card_exp_year,
                "card_holder_name": card_holder_name,
            }
        )
        if nick_name is not UNSET:
            field_dict["nick_name"] = nick_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_exp_month = d.pop("card_exp_month")

        card_exp_year = d.pop("card_exp_year")

        card_holder_name = d.pop("card_holder_name")

        def _parse_nick_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        nick_name = _parse_nick_name(d.pop("nick_name", UNSET))

        card_detail_update = cls(
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
            nick_name=nick_name,
        )

        return card_detail_update
