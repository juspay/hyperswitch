from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.adyen_split_data import AdyenSplitData


T = TypeVar("T", bound="SplitPaymentsRequestType1")


@_attrs_define
class SplitPaymentsRequestType1:
    """
    Attributes:
        adyen_split_payment (AdyenSplitData): Fee information for Split Payments to be charged on the payment being
            collected for Adyen
    """

    adyen_split_payment: "AdyenSplitData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        adyen_split_payment = self.adyen_split_payment.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "adyen_split_payment": adyen_split_payment,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.adyen_split_data import AdyenSplitData

        d = dict(src_dict)
        adyen_split_payment = AdyenSplitData.from_dict(d.pop("adyen_split_payment"))

        split_payments_request_type_1 = cls(
            adyen_split_payment=adyen_split_payment,
        )

        split_payments_request_type_1.additional_properties = d
        return split_payments_request_type_1

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
