from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..types import UNSET, Unset

T = TypeVar("T", bound="CardAdditionalData")


@_attrs_define
class CardAdditionalData:
    """Masked payout method details for card payout method

    Attributes:
        card_exp_month (str): Card expiry month Example: 01.
        card_exp_year (str): Card expiry year Example: 2026.
        card_holder_name (str): Card holder name Example: John Doe.
        card_issuer (Union[None, Unset, str]): Issuer of the card
        card_network (Union[CardNetwork, None, Unset]):
        card_type (Union[None, Unset, str]): Card type, can be either `credit` or `debit`
        card_issuing_country (Union[None, Unset, str]): Card issuing country
        bank_code (Union[None, Unset, str]): Code for Card issuing bank
        last4 (Union[None, Unset, str]): Last 4 digits of the card number
        card_isin (Union[None, Unset, str]): The ISIN of the card
        card_extended_bin (Union[None, Unset, str]): Extended bin of card, contains the first 8 digits of card number
    """

    card_exp_month: str
    card_exp_year: str
    card_holder_name: str
    card_issuer: Union[None, Unset, str] = UNSET
    card_network: Union[CardNetwork, None, Unset] = UNSET
    card_type: Union[None, Unset, str] = UNSET
    card_issuing_country: Union[None, Unset, str] = UNSET
    bank_code: Union[None, Unset, str] = UNSET
    last4: Union[None, Unset, str] = UNSET
    card_isin: Union[None, Unset, str] = UNSET
    card_extended_bin: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_exp_month = self.card_exp_month

        card_exp_year = self.card_exp_year

        card_holder_name = self.card_holder_name

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

        last4: Union[None, Unset, str]
        if isinstance(self.last4, Unset):
            last4 = UNSET
        else:
            last4 = self.last4

        card_isin: Union[None, Unset, str]
        if isinstance(self.card_isin, Unset):
            card_isin = UNSET
        else:
            card_isin = self.card_isin

        card_extended_bin: Union[None, Unset, str]
        if isinstance(self.card_extended_bin, Unset):
            card_extended_bin = UNSET
        else:
            card_extended_bin = self.card_extended_bin

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_exp_month": card_exp_month,
                "card_exp_year": card_exp_year,
                "card_holder_name": card_holder_name,
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
        if last4 is not UNSET:
            field_dict["last4"] = last4
        if card_isin is not UNSET:
            field_dict["card_isin"] = card_isin
        if card_extended_bin is not UNSET:
            field_dict["card_extended_bin"] = card_extended_bin

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_exp_month = d.pop("card_exp_month")

        card_exp_year = d.pop("card_exp_year")

        card_holder_name = d.pop("card_holder_name")

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

        def _parse_last4(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        last4 = _parse_last4(d.pop("last4", UNSET))

        def _parse_card_isin(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_isin = _parse_card_isin(d.pop("card_isin", UNSET))

        def _parse_card_extended_bin(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        card_extended_bin = _parse_card_extended_bin(d.pop("card_extended_bin", UNSET))

        card_additional_data = cls(
            card_exp_month=card_exp_month,
            card_exp_year=card_exp_year,
            card_holder_name=card_holder_name,
            card_issuer=card_issuer,
            card_network=card_network,
            card_type=card_type,
            card_issuing_country=card_issuing_country,
            bank_code=bank_code,
            last4=last4,
            card_isin=card_isin,
            card_extended_bin=card_extended_bin,
        )

        card_additional_data.additional_properties = d
        return card_additional_data

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
