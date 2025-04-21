from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.capture_method import CaptureMethod
from ..models.country_alpha_3 import CountryAlpha3
from ..models.currency import Currency
from ..models.feature_status import FeatureStatus
from ..models.payment_method import PaymentMethod
from ..models.payment_method_type import PaymentMethodType
from ..types import UNSET, Unset

T = TypeVar("T", bound="SupportedPaymentMethod")


@_attrs_define
class SupportedPaymentMethod:
    """
    Attributes:
        payment_method (PaymentMethod): Indicates the type of payment method. Eg: 'card', 'wallet', etc.
        payment_method_type (PaymentMethodType): Indicates the sub type of payment method. Eg: 'google_pay' &
            'apple_pay' for wallets.
        payment_method_type_display_name (str): The display name of the payment method type
        mandates (FeatureStatus): The status of the feature
        refunds (FeatureStatus): The status of the feature
        supported_capture_methods (list[CaptureMethod]): List of supported capture methods supported by the payment
            method type
        supported_countries (Union[None, Unset, list[CountryAlpha3]]): List of countries supported by the payment method
            type via the connector
        supported_currencies (Union[None, Unset, list[Currency]]): List of currencies supported by the payment method
            type via the connector
    """

    payment_method: PaymentMethod
    payment_method_type: PaymentMethodType
    payment_method_type_display_name: str
    mandates: FeatureStatus
    refunds: FeatureStatus
    supported_capture_methods: list[CaptureMethod]
    supported_countries: Union[None, Unset, list[CountryAlpha3]] = UNSET
    supported_currencies: Union[None, Unset, list[Currency]] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_method = self.payment_method.value

        payment_method_type = self.payment_method_type.value

        payment_method_type_display_name = self.payment_method_type_display_name

        mandates = self.mandates.value

        refunds = self.refunds.value

        supported_capture_methods = []
        for supported_capture_methods_item_data in self.supported_capture_methods:
            supported_capture_methods_item = supported_capture_methods_item_data.value
            supported_capture_methods.append(supported_capture_methods_item)

        supported_countries: Union[None, Unset, list[str]]
        if isinstance(self.supported_countries, Unset):
            supported_countries = UNSET
        elif isinstance(self.supported_countries, list):
            supported_countries = []
            for supported_countries_type_0_item_data in self.supported_countries:
                supported_countries_type_0_item = supported_countries_type_0_item_data.value
                supported_countries.append(supported_countries_type_0_item)

        else:
            supported_countries = self.supported_countries

        supported_currencies: Union[None, Unset, list[str]]
        if isinstance(self.supported_currencies, Unset):
            supported_currencies = UNSET
        elif isinstance(self.supported_currencies, list):
            supported_currencies = []
            for supported_currencies_type_0_item_data in self.supported_currencies:
                supported_currencies_type_0_item = supported_currencies_type_0_item_data.value
                supported_currencies.append(supported_currencies_type_0_item)

        else:
            supported_currencies = self.supported_currencies

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_method": payment_method,
                "payment_method_type": payment_method_type,
                "payment_method_type_display_name": payment_method_type_display_name,
                "mandates": mandates,
                "refunds": refunds,
                "supported_capture_methods": supported_capture_methods,
            }
        )
        if supported_countries is not UNSET:
            field_dict["supported_countries"] = supported_countries
        if supported_currencies is not UNSET:
            field_dict["supported_currencies"] = supported_currencies

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payment_method = PaymentMethod(d.pop("payment_method"))

        payment_method_type = PaymentMethodType(d.pop("payment_method_type"))

        payment_method_type_display_name = d.pop("payment_method_type_display_name")

        mandates = FeatureStatus(d.pop("mandates"))

        refunds = FeatureStatus(d.pop("refunds"))

        supported_capture_methods = []
        _supported_capture_methods = d.pop("supported_capture_methods")
        for supported_capture_methods_item_data in _supported_capture_methods:
            supported_capture_methods_item = CaptureMethod(supported_capture_methods_item_data)

            supported_capture_methods.append(supported_capture_methods_item)

        def _parse_supported_countries(data: object) -> Union[None, Unset, list[CountryAlpha3]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                supported_countries_type_0 = []
                _supported_countries_type_0 = data
                for supported_countries_type_0_item_data in _supported_countries_type_0:
                    supported_countries_type_0_item = CountryAlpha3(supported_countries_type_0_item_data)

                    supported_countries_type_0.append(supported_countries_type_0_item)

                return supported_countries_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[CountryAlpha3]], data)

        supported_countries = _parse_supported_countries(d.pop("supported_countries", UNSET))

        def _parse_supported_currencies(data: object) -> Union[None, Unset, list[Currency]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                supported_currencies_type_0 = []
                _supported_currencies_type_0 = data
                for supported_currencies_type_0_item_data in _supported_currencies_type_0:
                    supported_currencies_type_0_item = Currency(supported_currencies_type_0_item_data)

                    supported_currencies_type_0.append(supported_currencies_type_0_item)

                return supported_currencies_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[Currency]], data)

        supported_currencies = _parse_supported_currencies(d.pop("supported_currencies", UNSET))

        supported_payment_method = cls(
            payment_method=payment_method,
            payment_method_type=payment_method_type,
            payment_method_type_display_name=payment_method_type_display_name,
            mandates=mandates,
            refunds=refunds,
            supported_capture_methods=supported_capture_methods,
            supported_countries=supported_countries,
            supported_currencies=supported_currencies,
        )

        supported_payment_method.additional_properties = d
        return supported_payment_method

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
