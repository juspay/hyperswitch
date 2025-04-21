from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.apple_pay_address_parameters import ApplePayAddressParameters
from ..models.country_alpha_2 import CountryAlpha2
from ..models.currency import Currency
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.amount_info import AmountInfo
    from ..models.apple_pay_recurring_payment_request import ApplePayRecurringPaymentRequest


T = TypeVar("T", bound="ApplePayPaymentRequest")


@_attrs_define
class ApplePayPaymentRequest:
    """
    Attributes:
        country_code (CountryAlpha2):
        currency_code (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United States
            Dollar.
        total (AmountInfo):
        merchant_capabilities (Union[None, Unset, list[str]]): The list of merchant capabilities(ex: whether capable of
            3ds or no-3ds)
        supported_networks (Union[None, Unset, list[str]]): The list of supported networks
        merchant_identifier (Union[None, Unset, str]):
        required_billing_contact_fields (Union[None, Unset, list[ApplePayAddressParameters]]):
        required_shipping_contact_fields (Union[None, Unset, list[ApplePayAddressParameters]]):
        recurring_payment_request (Union['ApplePayRecurringPaymentRequest', None, Unset]):
    """

    country_code: CountryAlpha2
    currency_code: Currency
    total: "AmountInfo"
    merchant_capabilities: Union[None, Unset, list[str]] = UNSET
    supported_networks: Union[None, Unset, list[str]] = UNSET
    merchant_identifier: Union[None, Unset, str] = UNSET
    required_billing_contact_fields: Union[None, Unset, list[ApplePayAddressParameters]] = UNSET
    required_shipping_contact_fields: Union[None, Unset, list[ApplePayAddressParameters]] = UNSET
    recurring_payment_request: Union["ApplePayRecurringPaymentRequest", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.apple_pay_recurring_payment_request import ApplePayRecurringPaymentRequest

        country_code = self.country_code.value

        currency_code = self.currency_code.value

        total = self.total.to_dict()

        merchant_capabilities: Union[None, Unset, list[str]]
        if isinstance(self.merchant_capabilities, Unset):
            merchant_capabilities = UNSET
        elif isinstance(self.merchant_capabilities, list):
            merchant_capabilities = self.merchant_capabilities

        else:
            merchant_capabilities = self.merchant_capabilities

        supported_networks: Union[None, Unset, list[str]]
        if isinstance(self.supported_networks, Unset):
            supported_networks = UNSET
        elif isinstance(self.supported_networks, list):
            supported_networks = self.supported_networks

        else:
            supported_networks = self.supported_networks

        merchant_identifier: Union[None, Unset, str]
        if isinstance(self.merchant_identifier, Unset):
            merchant_identifier = UNSET
        else:
            merchant_identifier = self.merchant_identifier

        required_billing_contact_fields: Union[None, Unset, list[str]]
        if isinstance(self.required_billing_contact_fields, Unset):
            required_billing_contact_fields = UNSET
        elif isinstance(self.required_billing_contact_fields, list):
            required_billing_contact_fields = []
            for componentsschemas_apple_pay_billing_contact_fields_item_data in self.required_billing_contact_fields:
                componentsschemas_apple_pay_billing_contact_fields_item = (
                    componentsschemas_apple_pay_billing_contact_fields_item_data.value
                )
                required_billing_contact_fields.append(componentsschemas_apple_pay_billing_contact_fields_item)

        else:
            required_billing_contact_fields = self.required_billing_contact_fields

        required_shipping_contact_fields: Union[None, Unset, list[str]]
        if isinstance(self.required_shipping_contact_fields, Unset):
            required_shipping_contact_fields = UNSET
        elif isinstance(self.required_shipping_contact_fields, list):
            required_shipping_contact_fields = []
            for componentsschemas_apple_pay_shipping_contact_fields_item_data in self.required_shipping_contact_fields:
                componentsschemas_apple_pay_shipping_contact_fields_item = (
                    componentsschemas_apple_pay_shipping_contact_fields_item_data.value
                )
                required_shipping_contact_fields.append(componentsschemas_apple_pay_shipping_contact_fields_item)

        else:
            required_shipping_contact_fields = self.required_shipping_contact_fields

        recurring_payment_request: Union[None, Unset, dict[str, Any]]
        if isinstance(self.recurring_payment_request, Unset):
            recurring_payment_request = UNSET
        elif isinstance(self.recurring_payment_request, ApplePayRecurringPaymentRequest):
            recurring_payment_request = self.recurring_payment_request.to_dict()
        else:
            recurring_payment_request = self.recurring_payment_request

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "country_code": country_code,
                "currency_code": currency_code,
                "total": total,
            }
        )
        if merchant_capabilities is not UNSET:
            field_dict["merchant_capabilities"] = merchant_capabilities
        if supported_networks is not UNSET:
            field_dict["supported_networks"] = supported_networks
        if merchant_identifier is not UNSET:
            field_dict["merchant_identifier"] = merchant_identifier
        if required_billing_contact_fields is not UNSET:
            field_dict["required_billing_contact_fields"] = required_billing_contact_fields
        if required_shipping_contact_fields is not UNSET:
            field_dict["required_shipping_contact_fields"] = required_shipping_contact_fields
        if recurring_payment_request is not UNSET:
            field_dict["recurring_payment_request"] = recurring_payment_request

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.amount_info import AmountInfo
        from ..models.apple_pay_recurring_payment_request import ApplePayRecurringPaymentRequest

        d = dict(src_dict)
        country_code = CountryAlpha2(d.pop("country_code"))

        currency_code = Currency(d.pop("currency_code"))

        total = AmountInfo.from_dict(d.pop("total"))

        def _parse_merchant_capabilities(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                merchant_capabilities_type_0 = cast(list[str], data)

                return merchant_capabilities_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        merchant_capabilities = _parse_merchant_capabilities(d.pop("merchant_capabilities", UNSET))

        def _parse_supported_networks(data: object) -> Union[None, Unset, list[str]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                supported_networks_type_0 = cast(list[str], data)

                return supported_networks_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[str]], data)

        supported_networks = _parse_supported_networks(d.pop("supported_networks", UNSET))

        def _parse_merchant_identifier(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_identifier = _parse_merchant_identifier(d.pop("merchant_identifier", UNSET))

        def _parse_required_billing_contact_fields(data: object) -> Union[None, Unset, list[ApplePayAddressParameters]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                required_billing_contact_fields_type_1 = []
                _required_billing_contact_fields_type_1 = data
                for (
                    componentsschemas_apple_pay_billing_contact_fields_item_data
                ) in _required_billing_contact_fields_type_1:
                    componentsschemas_apple_pay_billing_contact_fields_item = ApplePayAddressParameters(
                        componentsschemas_apple_pay_billing_contact_fields_item_data
                    )

                    required_billing_contact_fields_type_1.append(
                        componentsschemas_apple_pay_billing_contact_fields_item
                    )

                return required_billing_contact_fields_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[ApplePayAddressParameters]], data)

        required_billing_contact_fields = _parse_required_billing_contact_fields(
            d.pop("required_billing_contact_fields", UNSET)
        )

        def _parse_required_shipping_contact_fields(
            data: object,
        ) -> Union[None, Unset, list[ApplePayAddressParameters]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                required_shipping_contact_fields_type_1 = []
                _required_shipping_contact_fields_type_1 = data
                for (
                    componentsschemas_apple_pay_shipping_contact_fields_item_data
                ) in _required_shipping_contact_fields_type_1:
                    componentsschemas_apple_pay_shipping_contact_fields_item = ApplePayAddressParameters(
                        componentsschemas_apple_pay_shipping_contact_fields_item_data
                    )

                    required_shipping_contact_fields_type_1.append(
                        componentsschemas_apple_pay_shipping_contact_fields_item
                    )

                return required_shipping_contact_fields_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[ApplePayAddressParameters]], data)

        required_shipping_contact_fields = _parse_required_shipping_contact_fields(
            d.pop("required_shipping_contact_fields", UNSET)
        )

        def _parse_recurring_payment_request(data: object) -> Union["ApplePayRecurringPaymentRequest", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                recurring_payment_request_type_1 = ApplePayRecurringPaymentRequest.from_dict(data)

                return recurring_payment_request_type_1
            except:  # noqa: E722
                pass
            return cast(Union["ApplePayRecurringPaymentRequest", None, Unset], data)

        recurring_payment_request = _parse_recurring_payment_request(d.pop("recurring_payment_request", UNSET))

        apple_pay_payment_request = cls(
            country_code=country_code,
            currency_code=currency_code,
            total=total,
            merchant_capabilities=merchant_capabilities,
            supported_networks=supported_networks,
            merchant_identifier=merchant_identifier,
            required_billing_contact_fields=required_billing_contact_fields,
            required_shipping_contact_fields=required_shipping_contact_fields,
            recurring_payment_request=recurring_payment_request,
        )

        apple_pay_payment_request.additional_properties = d
        return apple_pay_payment_request

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
