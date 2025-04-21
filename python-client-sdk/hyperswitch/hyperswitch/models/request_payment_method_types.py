from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_network import CardNetwork
from ..models.payment_experience import PaymentExperience
from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.accepted_countries_type_0 import AcceptedCountriesType0
    from ..models.accepted_countries_type_1 import AcceptedCountriesType1
    from ..models.accepted_countries_type_2 import AcceptedCountriesType2
    from ..models.accepted_currencies_type_0 import AcceptedCurrenciesType0
    from ..models.accepted_currencies_type_1 import AcceptedCurrenciesType1
    from ..models.accepted_currencies_type_2 import AcceptedCurrenciesType2


T = TypeVar("T", bound="RequestPaymentMethodTypes")


@_attrs_define
class RequestPaymentMethodTypes:
    """
    Attributes:
        payment_method_type (PaymentMethodType): Indicates the sub type of payment method. Eg: 'google_pay' &
            'apple_pay' for wallets.
        recurring_enabled (bool): Boolean to enable recurring payments / mandates. Default is true. Default: True.
        installment_payment_enabled (bool): Boolean to enable installment / EMI / BNPL payments. Default is true.
            Default: True.
        payment_experience (Union[None, PaymentExperience, Unset]):
        card_networks (Union[None, Unset, list[CardNetwork]]):
        accepted_currencies (Union['AcceptedCurrenciesType0', 'AcceptedCurrenciesType1', 'AcceptedCurrenciesType2',
            None, Unset]):
        accepted_countries (Union['AcceptedCountriesType0', 'AcceptedCountriesType1', 'AcceptedCountriesType2', None,
            Unset]):
        minimum_amount (Union[None, Unset, int]):
        maximum_amount (Union[None, Unset, int]):
    """

    payment_method_type: PaymentMethodType
    recurring_enabled: bool = True
    installment_payment_enabled: bool = True
    payment_experience: Union[None, PaymentExperience, Unset] = UNSET
    card_networks: Union[None, Unset, list[CardNetwork]] = UNSET
    accepted_currencies: Union[
        "AcceptedCurrenciesType0", "AcceptedCurrenciesType1", "AcceptedCurrenciesType2", None, Unset
    ] = UNSET
    accepted_countries: Union[
        "AcceptedCountriesType0", "AcceptedCountriesType1", "AcceptedCountriesType2", None, Unset
    ] = UNSET
    minimum_amount: Union[None, Unset, int] = UNSET
    maximum_amount: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.accepted_countries_type_0 import AcceptedCountriesType0
        from ..models.accepted_countries_type_1 import AcceptedCountriesType1
        from ..models.accepted_countries_type_2 import AcceptedCountriesType2
        from ..models.accepted_currencies_type_0 import AcceptedCurrenciesType0
        from ..models.accepted_currencies_type_1 import AcceptedCurrenciesType1
        from ..models.accepted_currencies_type_2 import AcceptedCurrenciesType2

        payment_method_type = self.payment_method_type.value

        recurring_enabled = self.recurring_enabled

        installment_payment_enabled = self.installment_payment_enabled

        payment_experience: Union[None, Unset, str]
        if isinstance(self.payment_experience, Unset):
            payment_experience = UNSET
        elif isinstance(self.payment_experience, PaymentExperience):
            payment_experience = self.payment_experience.value
        else:
            payment_experience = self.payment_experience

        card_networks: Union[None, Unset, list[str]]
        if isinstance(self.card_networks, Unset):
            card_networks = UNSET
        elif isinstance(self.card_networks, list):
            card_networks = []
            for card_networks_type_0_item_data in self.card_networks:
                card_networks_type_0_item = card_networks_type_0_item_data.value
                card_networks.append(card_networks_type_0_item)

        else:
            card_networks = self.card_networks

        accepted_currencies: Union[None, Unset, dict[str, Any]]
        if isinstance(self.accepted_currencies, Unset):
            accepted_currencies = UNSET
        elif isinstance(self.accepted_currencies, AcceptedCurrenciesType0):
            accepted_currencies = self.accepted_currencies.to_dict()
        elif isinstance(self.accepted_currencies, AcceptedCurrenciesType1):
            accepted_currencies = self.accepted_currencies.to_dict()
        elif isinstance(self.accepted_currencies, AcceptedCurrenciesType2):
            accepted_currencies = self.accepted_currencies.to_dict()
        else:
            accepted_currencies = self.accepted_currencies

        accepted_countries: Union[None, Unset, dict[str, Any]]
        if isinstance(self.accepted_countries, Unset):
            accepted_countries = UNSET
        elif isinstance(self.accepted_countries, AcceptedCountriesType0):
            accepted_countries = self.accepted_countries.to_dict()
        elif isinstance(self.accepted_countries, AcceptedCountriesType1):
            accepted_countries = self.accepted_countries.to_dict()
        elif isinstance(self.accepted_countries, AcceptedCountriesType2):
            accepted_countries = self.accepted_countries.to_dict()
        else:
            accepted_countries = self.accepted_countries

        minimum_amount: Union[None, Unset, int]
        if isinstance(self.minimum_amount, Unset):
            minimum_amount = UNSET
        else:
            minimum_amount = self.minimum_amount

        maximum_amount: Union[None, Unset, int]
        if isinstance(self.maximum_amount, Unset):
            maximum_amount = UNSET
        else:
            maximum_amount = self.maximum_amount

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_method_type": payment_method_type,
                "recurring_enabled": recurring_enabled,
                "installment_payment_enabled": installment_payment_enabled,
            }
        )
        if payment_experience is not UNSET:
            field_dict["payment_experience"] = payment_experience
        if card_networks is not UNSET:
            field_dict["card_networks"] = card_networks
        if accepted_currencies is not UNSET:
            field_dict["accepted_currencies"] = accepted_currencies
        if accepted_countries is not UNSET:
            field_dict["accepted_countries"] = accepted_countries
        if minimum_amount is not UNSET:
            field_dict["minimum_amount"] = minimum_amount
        if maximum_amount is not UNSET:
            field_dict["maximum_amount"] = maximum_amount

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.accepted_countries_type_0 import AcceptedCountriesType0
        from ..models.accepted_countries_type_1 import AcceptedCountriesType1
        from ..models.accepted_countries_type_2 import AcceptedCountriesType2
        from ..models.accepted_currencies_type_0 import AcceptedCurrenciesType0
        from ..models.accepted_currencies_type_1 import AcceptedCurrenciesType1
        from ..models.accepted_currencies_type_2 import AcceptedCurrenciesType2

        d = dict(src_dict)
        payment_method_type = PaymentMethodType(d.pop("payment_method_type"))

        recurring_enabled = d.pop("recurring_enabled")

        installment_payment_enabled = d.pop("installment_payment_enabled")

        def _parse_payment_experience(data: object) -> Union[None, PaymentExperience, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                payment_experience_type_1 = PaymentExperience(data)

                return payment_experience_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, PaymentExperience, Unset], data)

        payment_experience = _parse_payment_experience(d.pop("payment_experience", UNSET))

        def _parse_card_networks(data: object) -> Union[None, Unset, list[CardNetwork]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                card_networks_type_0 = []
                _card_networks_type_0 = data
                for card_networks_type_0_item_data in _card_networks_type_0:
                    card_networks_type_0_item = CardNetwork(card_networks_type_0_item_data)

                    card_networks_type_0.append(card_networks_type_0_item)

                return card_networks_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[CardNetwork]], data)

        card_networks = _parse_card_networks(d.pop("card_networks", UNSET))

        def _parse_accepted_currencies(
            data: object,
        ) -> Union["AcceptedCurrenciesType0", "AcceptedCurrenciesType1", "AcceptedCurrenciesType2", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_accepted_currencies_type_0 = AcceptedCurrenciesType0.from_dict(data)

                return componentsschemas_accepted_currencies_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_accepted_currencies_type_1 = AcceptedCurrenciesType1.from_dict(data)

                return componentsschemas_accepted_currencies_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_accepted_currencies_type_2 = AcceptedCurrenciesType2.from_dict(data)

                return componentsschemas_accepted_currencies_type_2
            except:  # noqa: E722
                pass
            return cast(
                Union["AcceptedCurrenciesType0", "AcceptedCurrenciesType1", "AcceptedCurrenciesType2", None, Unset],
                data,
            )

        accepted_currencies = _parse_accepted_currencies(d.pop("accepted_currencies", UNSET))

        def _parse_accepted_countries(
            data: object,
        ) -> Union["AcceptedCountriesType0", "AcceptedCountriesType1", "AcceptedCountriesType2", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_accepted_countries_type_0 = AcceptedCountriesType0.from_dict(data)

                return componentsschemas_accepted_countries_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_accepted_countries_type_1 = AcceptedCountriesType1.from_dict(data)

                return componentsschemas_accepted_countries_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_accepted_countries_type_2 = AcceptedCountriesType2.from_dict(data)

                return componentsschemas_accepted_countries_type_2
            except:  # noqa: E722
                pass
            return cast(
                Union["AcceptedCountriesType0", "AcceptedCountriesType1", "AcceptedCountriesType2", None, Unset], data
            )

        accepted_countries = _parse_accepted_countries(d.pop("accepted_countries", UNSET))

        def _parse_minimum_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        minimum_amount = _parse_minimum_amount(d.pop("minimum_amount", UNSET))

        def _parse_maximum_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        maximum_amount = _parse_maximum_amount(d.pop("maximum_amount", UNSET))

        request_payment_method_types = cls(
            payment_method_type=payment_method_type,
            recurring_enabled=recurring_enabled,
            installment_payment_enabled=installment_payment_enabled,
            payment_experience=payment_experience,
            card_networks=card_networks,
            accepted_currencies=accepted_currencies,
            accepted_countries=accepted_countries,
            minimum_amount=minimum_amount,
            maximum_amount=maximum_amount,
        )

        request_payment_method_types.additional_properties = d
        return request_payment_method_types

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
