from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.outgoing_webhook_response_content_headers_type_0_item_item import (
        OutgoingWebhookResponseContentHeadersType0ItemItem,
    )


T = TypeVar("T", bound="OutgoingWebhookResponseContent")


@_attrs_define
class OutgoingWebhookResponseContent:
    """The response information (headers, body and status code) received for the webhook sent.

    Attributes:
        body (Union[None, Unset, str]): The response body received for the webhook sent.
        headers (Union[None, Unset, list[list['OutgoingWebhookResponseContentHeadersType0ItemItem']]]): The response
            headers received for the webhook sent. Example: [['content-type', 'application/json'], ['content-length',
            '1024']].
        status_code (Union[None, Unset, int]): The HTTP status code for the webhook sent. Example: 200.
        error_message (Union[None, Unset, str]): Error message in case any error occurred when trying to deliver the
            webhook. Example: 200.
    """

    body: Union[None, Unset, str] = UNSET
    headers: Union[None, Unset, list[list["OutgoingWebhookResponseContentHeadersType0ItemItem"]]] = UNSET
    status_code: Union[None, Unset, int] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        body: Union[None, Unset, str]
        if isinstance(self.body, Unset):
            body = UNSET
        else:
            body = self.body

        headers: Union[None, Unset, list[list[dict[str, Any]]]]
        if isinstance(self.headers, Unset):
            headers = UNSET
        elif isinstance(self.headers, list):
            headers = []
            for headers_type_0_item_data in self.headers:
                headers_type_0_item = []
                for headers_type_0_item_item_data in headers_type_0_item_data:
                    headers_type_0_item_item = headers_type_0_item_item_data.to_dict()
                    headers_type_0_item.append(headers_type_0_item_item)

                headers.append(headers_type_0_item)

        else:
            headers = self.headers

        status_code: Union[None, Unset, int]
        if isinstance(self.status_code, Unset):
            status_code = UNSET
        else:
            status_code = self.status_code

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update({})
        if body is not UNSET:
            field_dict["body"] = body
        if headers is not UNSET:
            field_dict["headers"] = headers
        if status_code is not UNSET:
            field_dict["status_code"] = status_code
        if error_message is not UNSET:
            field_dict["error_message"] = error_message

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.outgoing_webhook_response_content_headers_type_0_item_item import (
            OutgoingWebhookResponseContentHeadersType0ItemItem,
        )

        d = dict(src_dict)

        def _parse_body(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        body = _parse_body(d.pop("body", UNSET))

        def _parse_headers(
            data: object,
        ) -> Union[None, Unset, list[list["OutgoingWebhookResponseContentHeadersType0ItemItem"]]]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, list):
                    raise TypeError()
                headers_type_0 = []
                _headers_type_0 = data
                for headers_type_0_item_data in _headers_type_0:
                    headers_type_0_item = []
                    _headers_type_0_item = headers_type_0_item_data
                    for headers_type_0_item_item_data in _headers_type_0_item:
                        headers_type_0_item_item = OutgoingWebhookResponseContentHeadersType0ItemItem.from_dict(
                            headers_type_0_item_item_data
                        )

                        headers_type_0_item.append(headers_type_0_item_item)

                    headers_type_0.append(headers_type_0_item)

                return headers_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, list[list["OutgoingWebhookResponseContentHeadersType0ItemItem"]]], data)

        headers = _parse_headers(d.pop("headers", UNSET))

        def _parse_status_code(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        status_code = _parse_status_code(d.pop("status_code", UNSET))

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        outgoing_webhook_response_content = cls(
            body=body,
            headers=headers,
            status_code=status_code,
            error_message=error_message,
        )

        outgoing_webhook_response_content.additional_properties = d
        return outgoing_webhook_response_content

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
