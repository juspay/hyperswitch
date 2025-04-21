from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.samsung_pay_card_brand import SamsungPayCardBrand
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.samsung_pay_token_data import SamsungPayTokenData


T = TypeVar("T", bound="SamsungPayWebWalletData")


@_attrs_define
class SamsungPayWebWalletData:
    """
    Attributes:
        card_brand (SamsungPayCardBrand):
        card_last4digits (str): Last 4 digits of the card number
        field_3_d_s (SamsungPayTokenData):
        method (Union[None, Unset, str]): Specifies authentication method used
        recurring_payment (Union[None, Unset, bool]): Value if credential is enabled for recurring payment
    """

    card_brand: SamsungPayCardBrand
    card_last4digits: str
    field_3_d_s: "SamsungPayTokenData"
    method: Union[None, Unset, str] = UNSET
    recurring_payment: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_brand = self.card_brand.value

        card_last4digits = self.card_last4digits

        field_3_d_s = self.field_3_d_s.to_dict()

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
                "card_brand": card_brand,
                "card_last4digits": card_last4digits,
                "3_d_s": field_3_d_s,
            }
        )
        if method is not UNSET:
            field_dict["method"] = method
        if recurring_payment is not UNSET:
            field_dict["recurring_payment"] = recurring_payment

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.samsung_pay_token_data import SamsungPayTokenData

        d = dict(src_dict)
        card_brand = SamsungPayCardBrand(d.pop("card_brand"))

        card_last4digits = d.pop("card_last4digits")

        field_3_d_s = SamsungPayTokenData.from_dict(d.pop("3_d_s"))

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

        samsung_pay_web_wallet_data = cls(
            card_brand=card_brand,
            card_last4digits=card_last4digits,
            field_3_d_s=field_3_d_s,
            method=method,
            recurring_payment=recurring_payment,
        )

        samsung_pay_web_wallet_data.additional_properties = d
        return samsung_pay_web_wallet_data

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
