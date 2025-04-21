from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

if TYPE_CHECKING:
    from ..models.outgoing_webhook_request_content_headers_item_item import OutgoingWebhookRequestContentHeadersItemItem


T = TypeVar("T", bound="OutgoingWebhookRequestContent")


@_attrs_define
class OutgoingWebhookRequestContent:
    """The request information (headers and body) sent in the webhook.

    Attributes:
        body (str): The request body sent in the webhook.
        headers (list[list['OutgoingWebhookRequestContentHeadersItemItem']]): The request headers sent in the webhook.
            Example: [['content-type', 'application/json'], ['content-length', '1024']].
    """

    body: str
    headers: list[list["OutgoingWebhookRequestContentHeadersItemItem"]]
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        body = self.body

        headers = []
        for headers_item_data in self.headers:
            headers_item = []
            for headers_item_item_data in headers_item_data:
                headers_item_item = headers_item_item_data.to_dict()
                headers_item.append(headers_item_item)

            headers.append(headers_item)

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "body": body,
                "headers": headers,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.outgoing_webhook_request_content_headers_item_item import (
            OutgoingWebhookRequestContentHeadersItemItem,
        )

        d = dict(src_dict)
        body = d.pop("body")

        headers = []
        _headers = d.pop("headers")
        for headers_item_data in _headers:
            headers_item = []
            _headers_item = headers_item_data
            for headers_item_item_data in _headers_item:
                headers_item_item = OutgoingWebhookRequestContentHeadersItemItem.from_dict(headers_item_item_data)

                headers_item.append(headers_item_item)

            headers.append(headers_item)

        outgoing_webhook_request_content = cls(
            body=body,
            headers=headers,
        )

        outgoing_webhook_request_content.additional_properties = d
        return outgoing_webhook_request_content

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
