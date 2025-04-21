from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define

T = TypeVar("T", bound="XenditSplitSubMerchantData")


@_attrs_define
class XenditSplitSubMerchantData:
    """Fee information to be charged on the payment being collected for sub-merchant via xendit

    Attributes:
        for_user_id (str): The sub-account user-id that you want to make this transaction for.
    """

    for_user_id: str

    def to_dict(self) -> dict[str, Any]:
        for_user_id = self.for_user_id

        field_dict: dict[str, Any] = {}
        field_dict.update(
            {
                "for_user_id": for_user_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        for_user_id = d.pop("for_user_id")

        xendit_split_sub_merchant_data = cls(
            for_user_id=for_user_id,
        )

        return xendit_split_sub_merchant_data
