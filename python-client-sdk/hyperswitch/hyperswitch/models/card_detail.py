from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..models.card_network import CardNetwork
from ..types import UNSET, Unset

T = TypeVar("T", bound="CardDetail")


@_attrs_define
class CardDetail:
    """
    Attributes:
        card_number (str): Card Number Example: 4111111145551142.
        card_exp_month (str): Card Expiry Month Example: 10.
        card_exp_year (str): Card Expiry Year Example: 25.
        card_holder_name (str): Card Holder Name Example: John Doe.
        nick_name (Union[None, Unset, str]): Card Holder's Nick Name Example: John Doe.
        card_issuing_country (Union[None, Unset, str]): Card Issuing Country
        card_network (Union[CardNetwork, None, Unset]):
        card_issuer (Union[None, Unset, str]): Issuer Bank for Card
        card_type (Union[None, Unset, str]): Card Type
    """

    card_number: str
    card_exp_month: str
    card_exp_year: str
    card_holder_name: str
    nick_name: Union[None, Unset, str] = UNSET
    card_issuing_country: Union[None, Unset, str] = UNSET
    card_network: Union[CardNetwork, None, Unset] = UNSET
    card_issuer: Union[None, Unset, str] = UNSET
    card_type: Union[None, Unset, str] = UNSET

    def to_dict(self) -> dict[str, Any]:
        card_number = self.card_number

        card_exp_month = self.card_exp_month

        card_exp_year = self.card_exp_year

        card_holder_name = self.card_holder_name

        nick_name: Union[None, Unset, str]
        if isinstance(self.nick_name, Unset):
            nick_name = UNSET
        else:
            nick_name = self.nick_name

        card_issuing_country: Union[None, Unset, str]
        if isinstance(self.card_issuing_country, Unset):
            card_issuing_country = UNSET
        else:
            card_issuing_country = self.card_issuing_country

        card_network: Union[None, Unset, str]
        if isinstance(self.card_network, Unset):
            card_network = UNSET
        elif isinstance(self.card_network, CardNetwork):
            card_network = self.card_network.value
        else:
            card_network = self.card_network

        card_issuer: Union[None, Unset, str]
        if isinstance(self.card_issuer, Unset):
            card_issuer = UNSET
        else:
            card_issuer = self.card_issuer

        card_type: Union[None, Unset, str]
        if isinstance(self.card_type, Unset):
            card_type = UNSET
        else:
            card_type = self.card_type

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "card_number": card_number,
                "card_exp_month": card_exp_month,
                "card_exp_year": card_exp_year,
                "card_holder_name": card_holder_name,
            }
        )
        if nick_name is not UNSET:
            field_dict["nick_name"] = nick_name
        if card_issuing_country is not UNSET:
            field_dict["card_issuing_country"] = card_issuing_country
        if card_network is not UNSET:
            field_dict["card_network"] = card_network
        if card_issuer is not UNSET:
            field_dict["card_issuer"] = card_issuer
        if card_type is not UNSET:
            field_dict["card_type"] = card_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_number = d.pop("card_number")

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

        def _parse_card_issuing_country(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_issuing_country = _parse_card_issuing_country(d.pop("card_issuing_country", UNSET))

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

        def _parse_card_issuer(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_issuer = _parse_card_issuer(d.pop("card_issuer", UNSET))

        def _parse_card_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_type = _parse_card_type(d.pop("card_type", UNSET))

        card_detail = cls(
            card_number=card_number,
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
            nick_name=nick_name,
            card_issuing_country=card_issuing_country,
            card_network=card_network,
            card_issuer=card_issuer,
            card_type=card_type,
        )

        return card_detail
