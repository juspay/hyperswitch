from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.mandate_status import MandateStatus
from ..types import UNSET, Unset

T = TypeVar("T", bound="MandateRevokedResponse")


@_attrs_define
class MandateRevokedResponse:
    """
    Attributes:
        mandate_id (str): The identifier for mandate
        status (MandateStatus): The status of the mandate, which indicates whether it can be used to initiate a payment.
        error_code (Union[None, Unset, str]): If there was an error while calling the connectors the code is received
            here Example: E0001.
        error_message (Union[None, Unset, str]): If there was an error while calling the connector the error message is
            received here Example: Failed while verifying the card.
    """

    mandate_id: str
    status: MandateStatus
    error_code: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        mandate_id = self.mandate_id

        status = self.status.value

        error_code: Union[None, Unset, str]
        if isinstance(self.error_code, Unset):
            error_code = UNSET
        else:
            error_code = self.error_code

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "mandate_id": mandate_id,
                "status": status,
            }
        )
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if error_message is not UNSET:
            field_dict["error_message"] = error_message

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        mandate_id = d.pop("mandate_id")

        status = MandateStatus(d.pop("status"))

        def _parse_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_code = _parse_error_code(d.pop("error_code", UNSET))

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        mandate_revoked_response = cls(
            mandate_id=mandate_id,
            status=status,
            error_code=error_code,
            error_message=error_message,
        )

        mandate_revoked_response.additional_properties = d
        return mandate_revoked_response

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
