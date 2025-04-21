import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.currency import Currency
from ..models.payment_link_status import PaymentLinkStatus
from ..types import UNSET, Unset

T = TypeVar("T", bound="RetrievePaymentLinkResponse")


@_attrs_define
class RetrievePaymentLinkResponse:
    """
    Attributes:
        payment_link_id (str): Identifier for Payment Link
        merchant_id (str): Identifier for Merchant
        link_to_pay (str): Open payment link (without any security checks and listing SPMs)
        amount (int): The payment amount. Amount for the payment in the lowest denomination of the currency Example:
            6540.
        created_at (datetime.datetime): Date and time of Payment Link creation
        status (PaymentLinkStatus): Status Of the Payment Link
        expiry (Union[None, Unset, datetime.datetime]): Date and time of Expiration for Payment Link
        description (Union[None, Unset, str]): Description for Payment Link
        currency (Union[Currency, None, Unset]):
        secure_link (Union[None, Unset, str]): Secure payment link (with security checks and listing saved payment
            methods)
    """

    payment_link_id: str
    merchant_id: str
    link_to_pay: str
    amount: int
    created_at: datetime.datetime
    status: PaymentLinkStatus
    expiry: Union[None, Unset, datetime.datetime] = UNSET
    description: Union[None, Unset, str] = UNSET
    currency: Union[Currency, None, Unset] = UNSET
    secure_link: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_link_id = self.payment_link_id

        merchant_id = self.merchant_id

        link_to_pay = self.link_to_pay

        amount = self.amount

        created_at = self.created_at.isoformat()

        status = self.status.value

        expiry: Union[None, Unset, str]
        if isinstance(self.expiry, Unset):
            expiry = UNSET
        elif isinstance(self.expiry, datetime.datetime):
            expiry = self.expiry.isoformat()
        else:
            expiry = self.expiry

        description: Union[None, Unset, str]
        if isinstance(self.description, Unset):
            description = UNSET
        else:
            description = self.description

        currency: Union[None, Unset, str]
        if isinstance(self.currency, Unset):
            currency = UNSET
        elif isinstance(self.currency, Currency):
            currency = self.currency.value
        else:
            currency = self.currency

        secure_link: Union[None, Unset, str]
        if isinstance(self.secure_link, Unset):
            secure_link = UNSET
        else:
            secure_link = self.secure_link

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_link_id": payment_link_id,
                "merchant_id": merchant_id,
                "link_to_pay": link_to_pay,
                "amount": amount,
                "created_at": created_at,
                "status": status,
            }
        )
        if expiry is not UNSET:
            field_dict["expiry"] = expiry
        if description is not UNSET:
            field_dict["description"] = description
        if currency is not UNSET:
            field_dict["currency"] = currency
        if secure_link is not UNSET:
            field_dict["secure_link"] = secure_link

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payment_link_id = d.pop("payment_link_id")

        merchant_id = d.pop("merchant_id")

        link_to_pay = d.pop("link_to_pay")

        amount = d.pop("amount")

        created_at = isoparse(d.pop("created_at"))

        status = PaymentLinkStatus(d.pop("status"))

        def _parse_expiry(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                expiry_type_0 = isoparse(data)

                return expiry_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        expiry = _parse_expiry(d.pop("expiry", UNSET))

        def _parse_description(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        description = _parse_description(d.pop("description", UNSET))

        def _parse_currency(data: object) -> Union[Currency, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                currency_type_1 = Currency(data)

                return currency_type_1
            except:  # noqa: E722
                pass
            return cast(Union[Currency, None, Unset], data)

        currency = _parse_currency(d.pop("currency", UNSET))

        def _parse_secure_link(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        secure_link = _parse_secure_link(d.pop("secure_link", UNSET))

        retrieve_payment_link_response = cls(
            payment_link_id=payment_link_id,
            merchant_id=merchant_id,
            link_to_pay=link_to_pay,
            amount=amount,
            created_at=created_at,
            status=status,
            expiry=expiry,
            description=description,
            currency=currency,
            secure_link=secure_link,
        )

        retrieve_payment_link_response.additional_properties = d
        return retrieve_payment_link_response

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
