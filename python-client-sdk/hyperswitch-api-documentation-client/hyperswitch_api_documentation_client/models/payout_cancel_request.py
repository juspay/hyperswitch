from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="PayoutCancelRequest")


@_attrs_define
class PayoutCancelRequest:
    """
    Attributes:
        payout_id (str): Unique identifier for the payout. This ensures idempotency for multiple payouts
            that have been done by a single merchant. This field is auto generated and is returned in the API response.
            Example: 187282ab-40ef-47a9-9206-5099ba31e432.
    """

    payout_id: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payout_id = self.payout_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payout_id": payout_id,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payout_id = d.pop("payout_id")

        payout_cancel_request = cls(
            payout_id=payout_id,
        )

        payout_cancel_request.additional_properties = d
        return payout_cancel_request

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
