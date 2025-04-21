from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.xendit_split_sub_merchant_data import XenditSplitSubMerchantData


T = TypeVar("T", bound="XenditSplitRequestType1")


@_attrs_define
class XenditSplitRequestType1:
    """
    Attributes:
        single_split (XenditSplitSubMerchantData): Fee information to be charged on the payment being collected for sub-
            merchant via xendit
    """

    single_split: "XenditSplitSubMerchantData"
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        single_split = self.single_split.to_dict()

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "single_split": single_split,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.xendit_split_sub_merchant_data import XenditSplitSubMerchantData

        d = dict(src_dict)
        single_split = XenditSplitSubMerchantData.from_dict(d.pop("single_split"))

        xendit_split_request_type_1 = cls(
            single_split=single_split,
        )

        xendit_split_request_type_1.additional_properties = d
        return xendit_split_request_type_1

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
