from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..types import UNSET, Unset

T = TypeVar("T", bound="CardDetailFromLocker")


@_attrs_define
class CardDetailFromLocker:
    """
    Attributes:
        saved_to_locker (bool):
        scheme (Union[None, Unset, str]):
        issuer_country (Union[None, Unset, str]):
        last4_digits (Union[None, Unset, str]):
        expiry_month (Union[None, Unset, str]):
        expiry_year (Union[None, Unset, str]):
        card_token (Union[None, Unset, str]):
        card_holder_name (Union[None, Unset, str]):
        card_fingerprint (Union[None, Unset, str]):
        nick_name (Union[None, Unset, str]):
        card_network (Union[CardNetwork, None, Unset]):
        card_isin (Union[None, Unset, str]):
        card_issuer (Union[None, Unset, str]):
        card_type (Union[None, Unset, str]):
    """

    saved_to_locker: bool
    scheme: Union[None, Unset, str] = UNSET
    issuer_country: Union[None, Unset, str] = UNSET
    last4_digits: Union[None, Unset, str] = UNSET
    expiry_month: Union[None, Unset, str] = UNSET
    expiry_year: Union[None, Unset, str] = UNSET
    card_token: Union[None, Unset, str] = UNSET
    card_holder_name: Union[None, Unset, str] = UNSET
    card_fingerprint: Union[None, Unset, str] = UNSET
    nick_name: Union[None, Unset, str] = UNSET
    card_network: Union[CardNetwork, None, Unset] = UNSET
    card_isin: Union[None, Unset, str] = UNSET
    card_issuer: Union[None, Unset, str] = UNSET
    card_type: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        saved_to_locker = self.saved_to_locker

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

        last4_digits: Union[None, Unset, str]
        if isinstance(self.last4_digits, Unset):
            last4_digits = UNSET
        else:
            last4_digits = self.last4_digits

        expiry_month: Union[None, Unset, str]
        if isinstance(self.expiry_month, Unset):
            expiry_month = UNSET
        else:
            expiry_month = self.expiry_month

        expiry_year: Union[None, Unset, str]
        if isinstance(self.expiry_year, Unset):
            expiry_year = UNSET
        else:
            expiry_year = self.expiry_year

        card_token: Union[None, Unset, str]
        if isinstance(self.card_token, Unset):
            card_token = UNSET
        else:
            card_token = self.card_token

        card_holder_name: Union[None, Unset, str]
        if isinstance(self.card_holder_name, Unset):
            card_holder_name = UNSET
        else:
            card_holder_name = self.card_holder_name

        card_fingerprint: Union[None, Unset, str]
        if isinstance(self.card_fingerprint, Unset):
            card_fingerprint = UNSET
        else:
            card_fingerprint = self.card_fingerprint

        nick_name: Union[None, Unset, str]
        if isinstance(self.nick_name, Unset):
            nick_name = UNSET
        else:
            nick_name = self.nick_name

        card_network: Union[None, Unset, str]
        if isinstance(self.card_network, Unset):
            card_network = UNSET
        elif isinstance(self.card_network, CardNetwork):
            card_network = self.card_network.value
        else:
            card_network = self.card_network

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

        card_type: Union[None, Unset, str]
        if isinstance(self.card_type, Unset):
            card_type = UNSET
        else:
            card_type = self.card_type

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "saved_to_locker": saved_to_locker,
            }
        )
        if scheme is not UNSET:
            field_dict["scheme"] = scheme
        if issuer_country is not UNSET:
            field_dict["issuer_country"] = issuer_country
        if last4_digits is not UNSET:
            field_dict["last4_digits"] = last4_digits
        if expiry_month is not UNSET:
            field_dict["expiry_month"] = expiry_month
        if expiry_year is not UNSET:
            field_dict["expiry_year"] = expiry_year
        if card_token is not UNSET:
            field_dict["card_token"] = card_token
        if card_holder_name is not UNSET:
            field_dict["card_holder_name"] = card_holder_name
        if card_fingerprint is not UNSET:
            field_dict["card_fingerprint"] = card_fingerprint
        if nick_name is not UNSET:
            field_dict["nick_name"] = nick_name
        if card_network is not UNSET:
            field_dict["card_network"] = card_network
        if card_isin is not UNSET:
            field_dict["card_isin"] = card_isin
        if card_issuer is not UNSET:
            field_dict["card_issuer"] = card_issuer
        if card_type is not UNSET:
            field_dict["card_type"] = card_type

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        saved_to_locker = d.pop("saved_to_locker")

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

        def _parse_last4_digits(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        last4_digits = _parse_last4_digits(d.pop("last4_digits", UNSET))

        def _parse_expiry_month(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        expiry_month = _parse_expiry_month(d.pop("expiry_month", UNSET))

        def _parse_expiry_year(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        expiry_year = _parse_expiry_year(d.pop("expiry_year", UNSET))

        def _parse_card_token(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_token = _parse_card_token(d.pop("card_token", UNSET))

        def _parse_card_holder_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_holder_name = _parse_card_holder_name(d.pop("card_holder_name", UNSET))

        def _parse_card_fingerprint(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_fingerprint = _parse_card_fingerprint(d.pop("card_fingerprint", UNSET))

        def _parse_nick_name(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        nick_name = _parse_nick_name(d.pop("nick_name", UNSET))

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

        def _parse_card_type(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_type = _parse_card_type(d.pop("card_type", UNSET))

        card_detail_from_locker = cls(
            saved_to_locker=saved_to_locker,
            scheme=scheme,
            issuer_country=issuer_country,
            last4_digits=last4_digits,
            expiry_month=expiry_month,
            expiry_year=expiry_year,
            card_token=card_token,
            card_holder_name=card_holder_name,
            card_fingerprint=card_fingerprint,
            nick_name=nick_name,
            card_network=card_network,
            card_isin=card_isin,
            card_issuer=card_issuer,
            card_type=card_type,
        )

        card_detail_from_locker.additional_properties = d
        return card_detail_from_locker

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
