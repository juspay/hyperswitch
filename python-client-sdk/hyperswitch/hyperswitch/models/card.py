from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..types import UNSET, Unset

T = TypeVar("T", bound="Card")


@_attrs_define
class Card:
    """
    Attributes:
        card_number (str): The card number Example: 4242424242424242.
        card_exp_month (str): The card's expiry month Example: 24.
        card_exp_year (str): The card's expiry year Example: 24.
        card_holder_name (str): The card holder's name Example: John Test.
        card_cvc (str): The CVC number for the card Example: 242.
        card_issuer (Union[None, Unset, str]): The name of the issuer of card Example: chase.
        card_network (Union[CardNetwork, None, Unset]):
        card_type (Union[None, Unset, str]):  Example: CREDIT.
        card_issuing_country (Union[None, Unset, str]):  Example: INDIA.
        bank_code (Union[None, Unset, str]):  Example: JP_AMEX.
        nick_name (Union[None, Unset, str]): The card holder's nick name Example: John Test.
    """

    card_number: str
    card_exp_month: str
    card_exp_year: str
    card_holder_name: str
    card_cvc: str
    card_issuer: Union[None, Unset, str] = UNSET
    card_network: Union[CardNetwork, None, Unset] = UNSET
    card_type: Union[None, Unset, str] = UNSET
    card_issuing_country: Union[None, Unset, str] = UNSET
    bank_code: Union[None, Unset, str] = UNSET
    nick_name: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_number = self.card_number

        card_exp_month = self.card_exp_month

        card_exp_year = self.card_exp_year

        card_holder_name = self.card_holder_name

        card_cvc = self.card_cvc

        card_issuer: Union[None, Unset, str]
        if isinstance(self.card_issuer, Unset):
            card_issuer = UNSET
        else:
            card_issuer = self.card_issuer

        card_network: Union[None, Unset, str]
        if isinstance(self.card_network, Unset):
            card_network = UNSET
        elif isinstance(self.card_network, CardNetwork):
            card_network = self.card_network.value
        else:
            card_network = self.card_network

        card_type: Union[None, Unset, str]
        if isinstance(self.card_type, Unset):
            card_type = UNSET
        else:
            card_type = self.card_type

        card_issuing_country: Union[None, Unset, str]
        if isinstance(self.card_issuing_country, Unset):
            card_issuing_country = UNSET
        else:
            card_issuing_country = self.card_issuing_country

        bank_code: Union[None, Unset, str]
        if isinstance(self.bank_code, Unset):
            bank_code = UNSET
        else:
            bank_code = self.bank_code

        nick_name: Union[None, Unset, str]
        if isinstance(self.nick_name, Unset):
            nick_name = UNSET
        else:
            nick_name = self.nick_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_number": card_number,
                "card_exp_month": card_exp_month,
                "card_exp_year": card_exp_year,
                "card_holder_name": card_holder_name,
                "card_cvc": card_cvc,
            }
        )
        if card_issuer is not UNSET:
            field_dict["card_issuer"] = card_issuer
        if card_network is not UNSET:
            field_dict["card_network"] = card_network
        if card_type is not UNSET:
            field_dict["card_type"] = card_type
        if card_issuing_country is not UNSET:
            field_dict["card_issuing_country"] = card_issuing_country
        if bank_code is not UNSET:
            field_dict["bank_code"] = bank_code
        if nick_name is not UNSET:
            field_dict["nick_name"] = nick_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_number = d.pop("card_number")

        card_exp_month = d.pop("card_exp_month")

        card_exp_year = d.pop("card_exp_year")

        card_holder_name = d.pop("card_holder_name")

        card_cvc = d.pop("card_cvc")

        def _parse_card_issuer(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_issuer = _parse_card_issuer(d.pop("card_issuer", UNSET))

        def _parse_card_network(data: object) -> Union[CardNetwork, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                card_network_type_1 = CardNetwork(data)

                return card_network_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CardNetwork, None, Unset], data)

        card_network = _parse_card_network(d.pop("card_network", UNSET))

        def _parse_card_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_type = _parse_card_type(d.pop("card_type", UNSET))

        def _parse_card_issuing_country(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_issuing_country = _parse_card_issuing_country(d.pop("card_issuing_country", UNSET))

        def _parse_bank_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        bank_code = _parse_bank_code(d.pop("bank_code", UNSET))

        def _parse_nick_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        nick_name = _parse_nick_name(d.pop("nick_name", UNSET))

        card = cls(
            card_number=card_number,
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
            card_cvc=card_cvc,
            card_issuer=card_issuer,
            card_network=card_network,
            card_type=card_type,
            card_issuing_country=card_issuing_country,
            bank_code=bank_code,
            nick_name=nick_name,
        )

        card.additional_properties = d
        return card

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
