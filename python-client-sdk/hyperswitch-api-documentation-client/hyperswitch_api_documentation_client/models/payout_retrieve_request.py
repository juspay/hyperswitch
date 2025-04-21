from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PayoutRetrieveRequest")


@_attrs_define
class PayoutRetrieveRequest:
    """
    Attributes:
        payout_id (str): Unique identifier for the payout. This ensures idempotency for multiple payouts
            that have been done by a single merchant. This field is auto generated and is returned in the API response.
            Example: 187282ab-40ef-47a9-9206-5099ba31e432.
        force_sync (Union[None, Unset, bool]): `force_sync` with the connector to get payout details
            (defaults to false) Default: False. Example: True.
        merchant_id (Union[None, Unset, str]): The identifier for the Merchant Account.
    """

    payout_id: str
    force_sync: Union[None, Unset, bool] = False
    merchant_id: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        payout_id = self.payout_id

        force_sync: Union[None, Unset, bool]
        if isinstance(self.force_sync, Unset):
            force_sync = UNSET
        else:
            force_sync = self.force_sync

        merchant_id: Union[None, Unset, str]
        if isinstance(self.merchant_id, Unset):
            merchant_id = UNSET
        else:
            merchant_id = self.merchant_id

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "payout_id": payout_id,
            }
        )
        if force_sync is not UNSET:
            field_dict["force_sync"] = force_sync
        if merchant_id is not UNSET:
            field_dict["merchant_id"] = merchant_id

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        payout_id = d.pop("payout_id")

        def _parse_force_sync(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        force_sync = _parse_force_sync(d.pop("force_sync", UNSET))

        def _parse_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_id = _parse_merchant_id(d.pop("merchant_id", UNSET))

        payout_retrieve_request = cls(
            payout_id=payout_id,
            force_sync=force_sync,
            merchant_id=merchant_id,
        )

        payout_retrieve_request.additional_properties = d
        return payout_retrieve_request

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
