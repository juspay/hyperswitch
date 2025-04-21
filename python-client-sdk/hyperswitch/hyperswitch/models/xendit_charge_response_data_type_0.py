from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.xendit_multiple_split_response import XenditMultipleSplitResponse


T = TypeVar("T", bound="XenditChargeResponseDataType0")


@_attrs_define
class XenditChargeResponseDataType0:
    """
    Attributes:
        multiple_splits (XenditMultipleSplitResponse): Fee information charged on the payment being collected via xendit
    """

    multiple_splits: "XenditMultipleSplitResponse"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        multiple_splits = self.multiple_splits.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "multiple_splits": multiple_splits,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.xendit_multiple_split_response import XenditMultipleSplitResponse

        d = dict(src_dict)
        multiple_splits = XenditMultipleSplitResponse.from_dict(d.pop("multiple_splits"))

        xendit_charge_response_data_type_0 = cls(
            multiple_splits=multiple_splits,
        )

        xendit_charge_response_data_type_0.additional_properties = d
        return xendit_charge_response_data_type_0

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
