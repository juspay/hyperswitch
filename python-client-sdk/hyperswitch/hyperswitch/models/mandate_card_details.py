from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..types import UNSET, Unset

T = TypeVar("T", bound="MandateCardDetails")


@_attrs_define
class MandateCardDetails:
    """
    Attributes:
        last4_digits (Union[None, Unset, str]): The last 4 digits of card
        card_exp_month (Union[None, Unset, str]): The expiry month of card
        card_exp_year (Union[None, Unset, str]): The expiry year of card
        card_holder_name (Union[None, Unset, str]): The card holder name
        card_token (Union[None, Unset, str]): The token from card locker
        scheme (Union[None, Unset, str]): The card scheme network for the particular card
        issuer_country (Union[None, Unset, str]): The country code in in which the card was issued
        card_fingerprint (Union[None, Unset, str]): A unique identifier alias to identify a particular card
        card_isin (Union[None, Unset, str]): The first 6 digits of card
        card_issuer (Union[None, Unset, str]): The bank that issued the card
        card_network (Union[CardNetwork, None, Unset]):
        card_type (Union[None, Unset, str]): The type of the payment card
        nick_name (Union[None, Unset, str]): The nick_name of the card holder
    """

    last4_digits: Union[None, Unset, str] = UNSET
    card_exp_month: Union[None, Unset, str] = UNSET
    card_exp_year: Union[None, Unset, str] = UNSET
    card_holder_name: Union[None, Unset, str] = UNSET
    card_token: Union[None, Unset, str] = UNSET
    scheme: Union[None, Unset, str] = UNSET
    issuer_country: Union[None, Unset, str] = UNSET
    card_fingerprint: Union[None, Unset, str] = UNSET
    card_isin: Union[None, Unset, str] = UNSET
    card_issuer: Union[None, Unset, str] = UNSET
    card_network: Union[CardNetwork, None, Unset] = UNSET
    card_type: Union[None, Unset, str] = UNSET
    nick_name: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        last4_digits: Union[None, Unset, str]
        if isinstance(self.last4_digits, Unset):
            last4_digits = UNSET
        else:
            last4_digits = self.last4_digits

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

        card_token: Union[None, Unset, str]
        if isinstance(self.card_token, Unset):
            card_token = UNSET
        else:
            card_token = self.card_token

        scheme: Union[None, Unset, str]
        if isinstance(self.scheme, Unset):
            scheme = UNSET
        else:
            scheme = self.scheme

        issuer_country: Union[None, Unset, str]
        if isinstance(self.issuer_country, Unset):
            issuer_country = UNSET
        else:
            issuer_country = self.issuer_country

        card_fingerprint: Union[None, Unset, str]
        if isinstance(self.card_fingerprint, Unset):
            card_fingerprint = UNSET
        else:
            card_fingerprint = self.card_fingerprint

        card_isin: Union[None, Unset, str]
        if isinstance(self.card_isin, Unset):
            card_isin = UNSET
        else:
            card_isin = self.card_isin

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

        nick_name: Union[None, Unset, str]
        if isinstance(self.nick_name, Unset):
            nick_name = UNSET
        else:
            nick_name = self.nick_name

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if last4_digits is not UNSET:
            field_dict["last4_digits"] = last4_digits
        if card_exp_month is not UNSET:
            field_dict["card_exp_month"] = card_exp_month
        if card_exp_year is not UNSET:
            field_dict["card_exp_year"] = card_exp_year
        if card_holder_name is not UNSET:
            field_dict["card_holder_name"] = card_holder_name
        if card_token is not UNSET:
            field_dict["card_token"] = card_token
        if scheme is not UNSET:
            field_dict["scheme"] = scheme
        if issuer_country is not UNSET:
            field_dict["issuer_country"] = issuer_country
        if card_fingerprint is not UNSET:
            field_dict["card_fingerprint"] = card_fingerprint
        if card_isin is not UNSET:
            field_dict["card_isin"] = card_isin
        if card_issuer is not UNSET:
            field_dict["card_issuer"] = card_issuer
        if card_network is not UNSET:
            field_dict["card_network"] = card_network
        if card_type is not UNSET:
            field_dict["card_type"] = card_type
        if nick_name is not UNSET:
            field_dict["nick_name"] = nick_name

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_last4_digits(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        last4_digits = _parse_last4_digits(d.pop("last4_digits", UNSET))

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

        def _parse_card_token(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_token = _parse_card_token(d.pop("card_token", UNSET))

        def _parse_scheme(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        scheme = _parse_scheme(d.pop("scheme", UNSET))

        def _parse_issuer_country(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        issuer_country = _parse_issuer_country(d.pop("issuer_country", UNSET))

        def _parse_card_fingerprint(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_fingerprint = _parse_card_fingerprint(d.pop("card_fingerprint", UNSET))

        def _parse_card_isin(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_isin = _parse_card_isin(d.pop("card_isin", UNSET))

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

        def _parse_nick_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        nick_name = _parse_nick_name(d.pop("nick_name", UNSET))

        mandate_card_details = cls(
            last4_digits=last4_digits,
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
            card_token=card_token,
            scheme=scheme,
            issuer_country=issuer_country,
            card_fingerprint=card_fingerprint,
            card_isin=card_isin,
            card_issuer=card_issuer,
            card_network=card_network,
            card_type=card_type,
            nick_name=nick_name,
        )

        mandate_card_details.additional_properties = d
        return mandate_card_details

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
