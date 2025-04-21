from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.error_category import ErrorCategory
from ..types import UNSET, Unset

T = TypeVar("T", bound="GsmResponse")


@_attrs_define
class GsmResponse:
    """
    Attributes:
        connector (str): The connector through which payment has gone through
        flow (str): The flow in which the code and message occurred for a connector
        sub_flow (str): The sub_flow in which the code and message occurred  for a connector
        code (str): code received from the connector
        message (str): message received from the connector
        status (str): status provided by the router
        decision (str): decision to be taken for auto retries flow
        step_up_possible (bool): indicates if step_up retry is possible
        clear_pan_possible (bool): indicates if retry with pan is possible
        router_error (Union[None, Unset, str]): optional error provided by the router
        unified_code (Union[None, Unset, str]): error code unified across the connectors
        unified_message (Union[None, Unset, str]): error message unified across the connectors
        error_category (Union[ErrorCategory, None, Unset]):
    """

    connector: str
    flow: str
    sub_flow: str
    code: str
    message: str
    status: str
    decision: str
    step_up_possible: bool
    clear_pan_possible: bool
    router_error: Union[None, Unset, str] = UNSET
    unified_code: Union[None, Unset, str] = UNSET
    unified_message: Union[None, Unset, str] = UNSET
    error_category: Union[ErrorCategory, None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = self.connector

        flow = self.flow

        sub_flow = self.sub_flow

        code = self.code

        message = self.message

        status = self.status

        decision = self.decision

        step_up_possible = self.step_up_possible

        clear_pan_possible = self.clear_pan_possible

        router_error: Union[None, Unset, str]
        if isinstance(self.router_error, Unset):
            router_error = UNSET
        else:
            router_error = self.router_error

        unified_code: Union[None, Unset, str]
        if isinstance(self.unified_code, Unset):
            unified_code = UNSET
        else:
            unified_code = self.unified_code

        unified_message: Union[None, Unset, str]
        if isinstance(self.unified_message, Unset):
            unified_message = UNSET
        else:
            unified_message = self.unified_message

        error_category: Union[None, Unset, str]
        if isinstance(self.error_category, Unset):
            error_category = UNSET
        elif isinstance(self.error_category, ErrorCategory):
            error_category = self.error_category.value
        else:
            error_category = self.error_category

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "connector": connector,
                "flow": flow,
                "sub_flow": sub_flow,
                "code": code,
                "message": message,
                "status": status,
                "decision": decision,
                "step_up_possible": step_up_possible,
                "clear_pan_possible": clear_pan_possible,
            }
        )
        if router_error is not UNSET:
            field_dict["router_error"] = router_error
        if unified_code is not UNSET:
            field_dict["unified_code"] = unified_code
        if unified_message is not UNSET:
            field_dict["unified_message"] = unified_message
        if error_category is not UNSET:
            field_dict["error_category"] = error_category

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        connector = d.pop("connector")

        flow = d.pop("flow")

        sub_flow = d.pop("sub_flow")

        code = d.pop("code")

        message = d.pop("message")

        status = d.pop("status")

        decision = d.pop("decision")

        step_up_possible = d.pop("step_up_possible")

        clear_pan_possible = d.pop("clear_pan_possible")

        def _parse_router_error(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        router_error = _parse_router_error(d.pop("router_error", UNSET))

        def _parse_unified_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        unified_code = _parse_unified_code(d.pop("unified_code", UNSET))

        def _parse_unified_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        unified_message = _parse_unified_message(d.pop("unified_message", UNSET))

        def _parse_error_category(data: object) -> Union[ErrorCategory, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                error_category_type_1 = ErrorCategory(data)

                return error_category_type_1
            except:  # noqa: E722
                pass
            return cast(Union[ErrorCategory, None, Unset], data)

        error_category = _parse_error_category(d.pop("error_category", UNSET))

        gsm_response = cls(
            connector=connector,
            flow=flow,
            sub_flow=sub_flow,
            code=code,
            message=message,
            status=status,
            decision=decision,
            step_up_possible=step_up_possible,
            clear_pan_possible=clear_pan_possible,
            router_error=router_error,
            unified_code=unified_code,
            unified_message=unified_message,
            error_category=error_category,
        )

        gsm_response.additional_properties = d
        return gsm_response

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
