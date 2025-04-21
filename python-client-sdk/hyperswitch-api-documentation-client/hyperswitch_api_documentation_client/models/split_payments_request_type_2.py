from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.xendit_split_request_type_0 import XenditSplitRequestType0
    from ..models.xendit_split_request_type_1 import XenditSplitRequestType1


T = TypeVar("T", bound="SplitPaymentsRequestType2")


@_attrs_define
class SplitPaymentsRequestType2:
    """
    Attributes:
        xendit_split_payment (Union['XenditSplitRequestType0', 'XenditSplitRequestType1']): Xendit Charge Request
    """

    xendit_split_payment: Union["XenditSplitRequestType0", "XenditSplitRequestType1"]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.xendit_split_request_type_0 import XenditSplitRequestType0

        xendit_split_payment: dict[str, Any]
        if isinstance(self.xendit_split_payment, XenditSplitRequestType0):
            xendit_split_payment = self.xendit_split_payment.to_dict()
        else:
            xendit_split_payment = self.xendit_split_payment.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "xendit_split_payment": xendit_split_payment,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.xendit_split_request_type_0 import XenditSplitRequestType0
        from ..models.xendit_split_request_type_1 import XenditSplitRequestType1

        d = dict(src_dict)

        def _parse_xendit_split_payment(data: object) -> Union["XenditSplitRequestType0", "XenditSplitRequestType1"]:
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_xendit_split_request_type_0 = XenditSplitRequestType0.from_dict(data)

                return componentsschemas_xendit_split_request_type_0
            except:  # noqa: E722
                pass
            if not isinstance(data, dict):
                raise TypeError()
            componentsschemas_xendit_split_request_type_1 = XenditSplitRequestType1.from_dict(data)

            return componentsschemas_xendit_split_request_type_1

        xendit_split_payment = _parse_xendit_split_payment(d.pop("xendit_split_payment"))

        split_payments_request_type_2 = cls(
            xendit_split_payment=xendit_split_payment,
        )

        split_payments_request_type_2.additional_properties = d
        return split_payments_request_type_2

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
