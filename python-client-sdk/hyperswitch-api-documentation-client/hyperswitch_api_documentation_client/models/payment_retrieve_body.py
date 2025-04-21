from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

T = TypeVar("T", bound="PaymentRetrieveBody")


@_attrs_define
class PaymentRetrieveBody:
    """
    Attributes:
        merchant_id (Union[None, Unset, str]): The identifier for the Merchant Account.
        force_sync (Union[None, Unset, bool]): Decider to enable or disable the connector call for retrieve request
        client_secret (Union[None, Unset, str]): This is a token which expires after 15 minutes, used from the client to
            authenticate and create sessions from the SDK
        expand_captures (Union[None, Unset, bool]): If enabled provides list of captures linked to latest attempt
        expand_attempts (Union[None, Unset, bool]): If enabled provides list of attempts linked to payment intent
    """

    merchant_id: Union[None, Unset, str] = UNSET
    force_sync: Union[None, Unset, bool] = UNSET
    client_secret: Union[None, Unset, str] = UNSET
    expand_captures: Union[None, Unset, bool] = UNSET
    expand_attempts: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        merchant_id: Union[None, Unset, str]
        if isinstance(self.merchant_id, Unset):
            merchant_id = UNSET
        else:
            merchant_id = self.merchant_id

        force_sync: Union[None, Unset, bool]
        if isinstance(self.force_sync, Unset):
            force_sync = UNSET
        else:
            force_sync = self.force_sync

        client_secret: Union[None, Unset, str]
        if isinstance(self.client_secret, Unset):
            client_secret = UNSET
        else:
            client_secret = self.client_secret

        expand_captures: Union[None, Unset, bool]
        if isinstance(self.expand_captures, Unset):
            expand_captures = UNSET
        else:
            expand_captures = self.expand_captures

        expand_attempts: Union[None, Unset, bool]
        if isinstance(self.expand_attempts, Unset):
            expand_attempts = UNSET
        else:
            expand_attempts = self.expand_attempts

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if merchant_id is not UNSET:
            field_dict["merchant_id"] = merchant_id
        if force_sync is not UNSET:
            field_dict["force_sync"] = force_sync
        if client_secret is not UNSET:
            field_dict["client_secret"] = client_secret
        if expand_captures is not UNSET:
            field_dict["expand_captures"] = expand_captures
        if expand_attempts is not UNSET:
            field_dict["expand_attempts"] = expand_attempts

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_merchant_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_id = _parse_merchant_id(d.pop("merchant_id", UNSET))

        def _parse_force_sync(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        force_sync = _parse_force_sync(d.pop("force_sync", UNSET))

        def _parse_client_secret(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        client_secret = _parse_client_secret(d.pop("client_secret", UNSET))

        def _parse_expand_captures(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        expand_captures = _parse_expand_captures(d.pop("expand_captures", UNSET))

        def _parse_expand_attempts(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        expand_attempts = _parse_expand_attempts(d.pop("expand_attempts", UNSET))

        payment_retrieve_body = cls(
            merchant_id=merchant_id,
            force_sync=force_sync,
            client_secret=client_secret,
            expand_captures=expand_captures,
            expand_attempts=expand_attempts,
        )

        payment_retrieve_body.additional_properties = d
        return payment_retrieve_body

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
