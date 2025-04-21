from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.recurring_details_type_2_type import RecurringDetailsType2Type

if TYPE_CHECKING:
    from ..models.processor_payment_token import ProcessorPaymentToken


T = TypeVar("T", bound="RecurringDetailsType2")


@_attrs_define
class RecurringDetailsType2:
    """
    Attributes:
        type_ (RecurringDetailsType2Type):
        data (ProcessorPaymentToken): Processor payment token for MIT payments where payment_method_data is not
            available
    """

    type_: RecurringDetailsType2Type
    data: "ProcessorPaymentToken"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        type_ = self.type_.value

        data = self.data.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "type": type_,
                "data": data,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.processor_payment_token import ProcessorPaymentToken

        d = dict(src_dict)
        type_ = RecurringDetailsType2Type(d.pop("type"))

        data = ProcessorPaymentToken.from_dict(d.pop("data"))

        recurring_details_type_2 = cls(
            type_=type_,
            data=data,
        )

        recurring_details_type_2.additional_properties = d
        return recurring_details_type_2

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
