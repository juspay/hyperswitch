from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.error_category import ErrorCategory
from ..models.gsm_decision import GsmDecision
from ..types import UNSET, Unset

T = TypeVar("T", bound="GsmUpdateRequest")


@_attrs_define
class GsmUpdateRequest:
    """
    Attributes:
        connector (str): The connector through which payment has gone through
        flow (str): The flow in which the code and message occurred for a connector
        sub_flow (str): The sub_flow in which the code and message occurred  for a connector
        code (str): code received from the connector
        message (str): message received from the connector
        status (Union[None, Unset, str]): status provided by the router
        router_error (Union[None, Unset, str]): optional error provided by the router
        decision (Union[GsmDecision, None, Unset]):
        step_up_possible (Union[None, Unset, bool]): indicates if step_up retry is possible
        unified_code (Union[None, Unset, str]): error code unified across the connectors
        unified_message (Union[None, Unset, str]): error message unified across the connectors
        error_category (Union[ErrorCategory, None, Unset]):
        clear_pan_possible (Union[None, Unset, bool]): indicates if retry with pan is possible
    """

    connector: str
    flow: str
    sub_flow: str
    code: str
    message: str
    status: Union[None, Unset, str] = UNSET
    router_error: Union[None, Unset, str] = UNSET
    decision: Union[GsmDecision, None, Unset] = UNSET
    step_up_possible: Union[None, Unset, bool] = UNSET
    unified_code: Union[None, Unset, str] = UNSET
    unified_message: Union[None, Unset, str] = UNSET
    error_category: Union[ErrorCategory, None, Unset] = UNSET
    clear_pan_possible: Union[None, Unset, bool] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        connector = self.connector

        flow = self.flow

        sub_flow = self.sub_flow

        code = self.code

        message = self.message

        status: Union[None, Unset, str]
        if isinstance(self.status, Unset):
            status = UNSET
        else:
            status = self.status

        router_error: Union[None, Unset, str]
        if isinstance(self.router_error, Unset):
            router_error = UNSET
        else:
            router_error = self.router_error

        decision: Union[None, Unset, str]
        if isinstance(self.decision, Unset):
            decision = UNSET
        elif isinstance(self.decision, GsmDecision):
            decision = self.decision.value
        else:
            decision = self.decision

        step_up_possible: Union[None, Unset, bool]
        if isinstance(self.step_up_possible, Unset):
            step_up_possible = UNSET
        else:
            step_up_possible = self.step_up_possible

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

        clear_pan_possible: Union[None, Unset, bool]
        if isinstance(self.clear_pan_possible, Unset):
            clear_pan_possible = UNSET
        else:
            clear_pan_possible = self.clear_pan_possible

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
        if status is not UNSET:
            field_dict["status"] = status
        if router_error is not UNSET:
            field_dict["router_error"] = router_error
        if decision is not UNSET:
            field_dict["decision"] = decision
        if step_up_possible is not UNSET:
            field_dict["step_up_possible"] = step_up_possible
        if unified_code is not UNSET:
            field_dict["unified_code"] = unified_code
        if unified_message is not UNSET:
            field_dict["unified_message"] = unified_message
        if error_category is not UNSET:
            field_dict["error_category"] = error_category
        if clear_pan_possible is not UNSET:
            field_dict["clear_pan_possible"] = clear_pan_possible

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        connector = d.pop("connector")

        flow = d.pop("flow")

        sub_flow = d.pop("sub_flow")

        code = d.pop("code")

        message = d.pop("message")

        def _parse_status(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        status = _parse_status(d.pop("status", UNSET))

        def _parse_router_error(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        router_error = _parse_router_error(d.pop("router_error", UNSET))

        def _parse_decision(data: object) -> Union[GsmDecision, None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                decision_type_1 = GsmDecision(data)

                return decision_type_1
            except:  # noqa: E722
                pass
            return cast(Union[GsmDecision, None, Unset], data)

        decision = _parse_decision(d.pop("decision", UNSET))

        def _parse_step_up_possible(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        step_up_possible = _parse_step_up_possible(d.pop("step_up_possible", UNSET))

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

        def _parse_clear_pan_possible(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        clear_pan_possible = _parse_clear_pan_possible(d.pop("clear_pan_possible", UNSET))

        gsm_update_request = cls(
            connector=connector,
            flow=flow,
            sub_flow=sub_flow,
            code=code,
            message=message,
            status=status,
            router_error=router_error,
            decision=decision,
            step_up_possible=step_up_possible,
            unified_code=unified_code,
            unified_message=unified_message,
            error_category=error_category,
            clear_pan_possible=clear_pan_possible,
        )

        gsm_update_request.additional_properties = d
        return gsm_update_request

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
