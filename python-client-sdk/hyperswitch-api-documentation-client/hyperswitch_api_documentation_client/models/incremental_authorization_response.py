from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.authorization_status import AuthorizationStatus
from ..types import UNSET, Unset

T = TypeVar("T", bound="IncrementalAuthorizationResponse")


@_attrs_define
class IncrementalAuthorizationResponse:
    """
    Attributes:
        authorization_id (str): The unique identifier of authorization
        amount (int): Amount the authorization has been made for Example: 6540.
        status (AuthorizationStatus):
        previously_authorized_amount (int): This Unit struct represents MinorUnit in which core amount works
        error_code (Union[None, Unset, str]): Error code sent by the connector for authorization
        error_message (Union[None, Unset, str]): Error message sent by the connector for authorization
    """

    authorization_id: str
    amount: int
    status: AuthorizationStatus
    previously_authorized_amount: int
    error_code: Union[None, Unset, str] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        authorization_id = self.authorization_id

        amount = self.amount

        status = self.status.value

        previously_authorized_amount = self.previously_authorized_amount

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
                "authorization_id": authorization_id,
                "amount": amount,
                "status": status,
                "previously_authorized_amount": previously_authorized_amount,
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
        authorization_id = d.pop("authorization_id")

        amount = d.pop("amount")

        status = AuthorizationStatus(d.pop("status"))

        previously_authorized_amount = d.pop("previously_authorized_amount")

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

        incremental_authorization_response = cls(
            authorization_id=authorization_id,
            amount=amount,
            status=status,
            previously_authorized_amount=previously_authorized_amount,
            error_code=error_code,
            error_message=error_message,
        )

        incremental_authorization_response.additional_properties = d
        return incremental_authorization_response

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
