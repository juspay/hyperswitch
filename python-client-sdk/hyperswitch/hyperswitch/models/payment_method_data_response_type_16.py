from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.mobile_payment_response import MobilePaymentResponse


T = TypeVar("T", bound="PaymentMethodDataResponseType16")


@_attrs_define
class PaymentMethodDataResponseType16:
    """
    Attributes:
        mobile_payment (MobilePaymentResponse):
    """

    mobile_payment: "MobilePaymentResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        mobile_payment = self.mobile_payment.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "mobile_payment": mobile_payment,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.mobile_payment_response import MobilePaymentResponse

        d = dict(src_dict)
        mobile_payment = MobilePaymentResponse.from_dict(d.pop("mobile_payment"))

        payment_method_data_response_type_16 = cls(
            mobile_payment=mobile_payment,
        )

        payment_method_data_response_type_16.additional_properties = d
        return payment_method_data_response_type_16

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
