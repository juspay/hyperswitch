from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.samsung_pay_card_brand import SamsungPayCardBrand
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.samsung_pay_token_data import SamsungPayTokenData


T = TypeVar("T", bound="SamsungPayAppWalletData")


@_attrs_define
class SamsungPayAppWalletData:
    """
    Attributes:
        field_3_d_s (SamsungPayTokenData):
        payment_card_brand (SamsungPayCardBrand):
        payment_currency_type (str): Currency type of the payment
        payment_last4_fpan (str): Last 4 digits of the card number
        payment_last4_dpan (Union[None, Unset, str]): Last 4 digits of the device specific card number
        merchant_ref (Union[None, Unset, str]): Merchant reference id that was passed in the session call request
        method (Union[None, Unset, str]): Specifies authentication method used
        recurring_payment (Union[None, Unset, bool]): Value if credential is enabled for recurring payment
    """

    field_3_d_s: "SamsungPayTokenData"
    payment_card_brand: SamsungPayCardBrand
    payment_currency_type: str
    payment_last4_fpan: str
    payment_last4_dpan: Union[None, Unset, str] = UNSET
    merchant_ref: Union[None, Unset, str] = UNSET
    method: Union[None, Unset, str] = UNSET
    recurring_payment: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        field_3_d_s = self.field_3_d_s.to_dict()

        payment_card_brand = self.payment_card_brand.value

        payment_currency_type = self.payment_currency_type

        payment_last4_fpan = self.payment_last4_fpan

        payment_last4_dpan: Union[None, Unset, str]
        if isinstance(self.payment_last4_dpan, Unset):
            payment_last4_dpan = UNSET
        else:
            payment_last4_dpan = self.payment_last4_dpan

        merchant_ref: Union[None, Unset, str]
        if isinstance(self.merchant_ref, Unset):
            merchant_ref = UNSET
        else:
            merchant_ref = self.merchant_ref

        method: Union[None, Unset, str]
        if isinstance(self.method, Unset):
            method = UNSET
        else:
            method = self.method

        recurring_payment: Union[None, Unset, bool]
        if isinstance(self.recurring_payment, Unset):
            recurring_payment = UNSET
        else:
            recurring_payment = self.recurring_payment

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "3_d_s": field_3_d_s,
                "payment_card_brand": payment_card_brand,
                "payment_currency_type": payment_currency_type,
                "payment_last4_fpan": payment_last4_fpan,
            }
        )
        if payment_last4_dpan is not UNSET:
            field_dict["payment_last4_dpan"] = payment_last4_dpan
        if merchant_ref is not UNSET:
            field_dict["merchant_ref"] = merchant_ref
        if method is not UNSET:
            field_dict["method"] = method
        if recurring_payment is not UNSET:
            field_dict["recurring_payment"] = recurring_payment

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.samsung_pay_token_data import SamsungPayTokenData

        d = dict(src_dict)
        field_3_d_s = SamsungPayTokenData.from_dict(d.pop("3_d_s"))

        payment_card_brand = SamsungPayCardBrand(d.pop("payment_card_brand"))

        payment_currency_type = d.pop("payment_currency_type")

        payment_last4_fpan = d.pop("payment_last4_fpan")

        def _parse_payment_last4_dpan(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        payment_last4_dpan = _parse_payment_last4_dpan(d.pop("payment_last4_dpan", UNSET))

        def _parse_merchant_ref(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_ref = _parse_merchant_ref(d.pop("merchant_ref", UNSET))

        def _parse_method(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        method = _parse_method(d.pop("method", UNSET))

        def _parse_recurring_payment(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        recurring_payment = _parse_recurring_payment(d.pop("recurring_payment", UNSET))

        samsung_pay_app_wallet_data = cls(
            field_3_d_s=field_3_d_s,
            payment_card_brand=payment_card_brand,
            payment_currency_type=payment_currency_type,
            payment_last4_fpan=payment_last4_fpan,
            payment_last4_dpan=payment_last4_dpan,
            merchant_ref=merchant_ref,
            method=method,
            recurring_payment=recurring_payment,
        )

        samsung_pay_app_wallet_data.additional_properties = d
        return samsung_pay_app_wallet_data

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
