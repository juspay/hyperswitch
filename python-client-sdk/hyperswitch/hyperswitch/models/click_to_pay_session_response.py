from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.ctp_service_provider import CtpServiceProvider
from ..models.currency import Currency
from ..types import UNSET, Unset

T = TypeVar("T", bound="ClickToPaySessionResponse")


@_attrs_define
class ClickToPaySessionResponse:
    """
    Attributes:
        dpa_id (str):
        dpa_name (str):
        locale (str):
        card_brands (list[str]):
        acquirer_bin (str):
        acquirer_merchant_id (str):
        merchant_category_code (str):
        merchant_country_code (str):
        transaction_amount (str):  Example: 38.02.
        transaction_currency_code (Currency): The three letter ISO currency code in uppercase. Eg: 'USD' for the United
            States Dollar.
        phone_number (Union[None, Unset, str]):  Example: 9123456789.
        email (Union[None, Unset, str]):  Example: johntest@test.com.
        phone_country_code (Union[None, Unset, str]):
        provider (Union[CtpServiceProvider, None, Unset]):
        dpa_client_id (Union[None, Unset, str]):
    """

    dpa_id: str
    dpa_name: str
    locale: str
    card_brands: list[str]
    acquirer_bin: str
    acquirer_merchant_id: str
    merchant_category_code: str
    merchant_country_code: str
    transaction_amount: str
    transaction_currency_code: Currency
    phone_number: Union[None, Unset, str] = UNSET
    email: Union[None, Unset, str] = UNSET
    phone_country_code: Union[None, Unset, str] = UNSET
    provider: Union[CtpServiceProvider, None, Unset] = UNSET
    dpa_client_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        dpa_id = self.dpa_id

        dpa_name = self.dpa_name

        locale = self.locale

        card_brands = self.card_brands

        acquirer_bin = self.acquirer_bin

        acquirer_merchant_id = self.acquirer_merchant_id

        merchant_category_code = self.merchant_category_code

        merchant_country_code = self.merchant_country_code

        transaction_amount = self.transaction_amount

        transaction_currency_code = self.transaction_currency_code.value

        phone_number: Union[None, Unset, str]
        if isinstance(self.phone_number, Unset):
            phone_number = UNSET
        else:
            phone_number = self.phone_number

        email: Union[None, Unset, str]
        if isinstance(self.email, Unset):
            email = UNSET
        else:
            email = self.email

        phone_country_code: Union[None, Unset, str]
        if isinstance(self.phone_country_code, Unset):
            phone_country_code = UNSET
        else:
            phone_country_code = self.phone_country_code

        provider: Union[None, Unset, str]
        if isinstance(self.provider, Unset):
            provider = UNSET
        elif isinstance(self.provider, CtpServiceProvider):
            provider = self.provider.value
        else:
            provider = self.provider

        dpa_client_id: Union[None, Unset, str]
        if isinstance(self.dpa_client_id, Unset):
            dpa_client_id = UNSET
        else:
            dpa_client_id = self.dpa_client_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "dpa_id": dpa_id,
                "dpa_name": dpa_name,
                "locale": locale,
                "card_brands": card_brands,
                "acquirer_bin": acquirer_bin,
                "acquirer_merchant_id": acquirer_merchant_id,
                "merchant_category_code": merchant_category_code,
                "merchant_country_code": merchant_country_code,
                "transaction_amount": transaction_amount,
                "transaction_currency_code": transaction_currency_code,
            }
        )
        if phone_number is not UNSET:
            field_dict["phone_number"] = phone_number
        if email is not UNSET:
            field_dict["email"] = email
        if phone_country_code is not UNSET:
            field_dict["phone_country_code"] = phone_country_code
        if provider is not UNSET:
            field_dict["provider"] = provider
        if dpa_client_id is not UNSET:
            field_dict["dpa_client_id"] = dpa_client_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        dpa_id = d.pop("dpa_id")

        dpa_name = d.pop("dpa_name")

        locale = d.pop("locale")

        card_brands = cast(list[str], d.pop("card_brands"))

        acquirer_bin = d.pop("acquirer_bin")

        acquirer_merchant_id = d.pop("acquirer_merchant_id")

        merchant_category_code = d.pop("merchant_category_code")

        merchant_country_code = d.pop("merchant_country_code")

        transaction_amount = d.pop("transaction_amount")

        transaction_currency_code = Currency(d.pop("transaction_currency_code"))

        def _parse_phone_number(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        phone_number = _parse_phone_number(d.pop("phone_number", UNSET))

        def _parse_email(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        email = _parse_email(d.pop("email", UNSET))

        def _parse_phone_country_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        phone_country_code = _parse_phone_country_code(d.pop("phone_country_code", UNSET))

        def _parse_provider(data: object) -> Union[CtpServiceProvider, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                provider_type_1 = CtpServiceProvider(data)

                return provider_type_1
            except:  # noqa: E722
                pass
            return cast(Union[CtpServiceProvider, None, Unset], data)

        provider = _parse_provider(d.pop("provider", UNSET))

        def _parse_dpa_client_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        dpa_client_id = _parse_dpa_client_id(d.pop("dpa_client_id", UNSET))

        click_to_pay_session_response = cls(
            dpa_id=dpa_id,
            dpa_name=dpa_name,
            locale=locale,
            card_brands=card_brands,
            acquirer_bin=acquirer_bin,
            acquirer_merchant_id=acquirer_merchant_id,
            merchant_category_code=merchant_category_code,
            merchant_country_code=merchant_country_code,
            transaction_amount=transaction_amount,
            transaction_currency_code=transaction_currency_code,
            phone_number=phone_number,
            email=email,
            phone_country_code=phone_country_code,
            provider=provider,
            dpa_client_id=dpa_client_id,
        )

        click_to_pay_session_response.additional_properties = d
        return click_to_pay_session_response

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
