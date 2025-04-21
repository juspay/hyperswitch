from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.display_amount_on_sdk import DisplayAmountOnSdk


T = TypeVar("T", bound="PaymentsDynamicTaxCalculationResponse")


@_attrs_define
class PaymentsDynamicTaxCalculationResponse:
    """
    Attributes:
        payment_id (str): The identifier for the payment
        net_amount (int): This Unit struct represents MinorUnit in which core amount works
        display_amount (DisplayAmountOnSdk):
        order_tax_amount (Union[None, Unset, int]):
        shipping_cost (Union[None, Unset, int]):
    """

    payment_id: str
    net_amount: int
    display_amount: "DisplayAmountOnSdk"
    order_tax_amount: Union[None, Unset, int] = UNSET
    shipping_cost: Union[None, Unset, int] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payment_id = self.payment_id

        net_amount = self.net_amount

        display_amount = self.display_amount.to_dict()

        order_tax_amount: Union[None, Unset, int]
        if isinstance(self.order_tax_amount, Unset):
            order_tax_amount = UNSET
        else:
            order_tax_amount = self.order_tax_amount

        shipping_cost: Union[None, Unset, int]
        if isinstance(self.shipping_cost, Unset):
            shipping_cost = UNSET
        else:
            shipping_cost = self.shipping_cost

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payment_id": payment_id,
                "net_amount": net_amount,
                "display_amount": display_amount,
            }
        )
        if order_tax_amount is not UNSET:
            field_dict["order_tax_amount"] = order_tax_amount
        if shipping_cost is not UNSET:
            field_dict["shipping_cost"] = shipping_cost

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.display_amount_on_sdk import DisplayAmountOnSdk

        d = dict(src_dict)
        payment_id = d.pop("payment_id")

        net_amount = d.pop("net_amount")

        display_amount = DisplayAmountOnSdk.from_dict(d.pop("display_amount"))

        def _parse_order_tax_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        order_tax_amount = _parse_order_tax_amount(d.pop("order_tax_amount", UNSET))

        def _parse_shipping_cost(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        shipping_cost = _parse_shipping_cost(d.pop("shipping_cost", UNSET))

        payments_dynamic_tax_calculation_response = cls(
            payment_id=payment_id,
            net_amount=net_amount,
            display_amount=display_amount,
            order_tax_amount=order_tax_amount,
            shipping_cost=shipping_cost,
        )

        payments_dynamic_tax_calculation_response.additional_properties = d
        return payments_dynamic_tax_calculation_response

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
