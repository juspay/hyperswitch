from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

T = TypeVar("T", bound="GsmDeleteRequest")


@_attrs_define
class GsmDeleteRequest:
    """
    Attributes:
        connector (str): The connector through which payment has gone through
        flow (str): The flow in which the code and message occurred for a connector
        sub_flow (str): The sub_flow in which the code and message occurred  for a connector
        code (str): code received from the connector
        message (str): message received from the connector
    """

    connector: str
    flow: str
    sub_flow: str
    code: str
    message: str
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = self.connector

        flow = self.flow

        sub_flow = self.sub_flow

        code = self.code

        message = self.message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
                "flow": flow,
                "sub_flow": sub_flow,
                "code": code,
                "message": message,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        connector = d.pop("connector")

        flow = d.pop("flow")

        sub_flow = d.pop("sub_flow")

        code = d.pop("code")

        message = d.pop("message")

        gsm_delete_request = cls(
            connector=connector,
            flow=flow,
            sub_flow=sub_flow,
            code=code,
            message=message,
        )

        gsm_delete_request.additional_properties = d
        return gsm_delete_request

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
