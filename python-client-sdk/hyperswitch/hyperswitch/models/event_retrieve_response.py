import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.event_class import EventClass
from ..models.event_type import EventType
from ..models.webhook_delivery_attempt import WebhookDeliveryAttempt
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.outgoing_webhook_request_content import OutgoingWebhookRequestContent
    from ..models.outgoing_webhook_response_content import OutgoingWebhookResponseContent


T = TypeVar("T", bound="EventRetrieveResponse")


@_attrs_define
class EventRetrieveResponse:
    """The response body for retrieving an event.

    Attributes:
        event_id (str): The identifier for the Event. Example: evt_018e31720d1b7a2b82677d3032cab959.
        merchant_id (str): The identifier for the Merchant Account. Example: y3oqhf46pyzuxjbcn2giaqnb44.
        profile_id (str): The identifier for the Business Profile. Example: SqB0zwDGR5wHppWf0bx7GKr1f2.
        object_id (str): The identifier for the object (Payment Intent ID, Refund ID, etc.) Example:
            QHrfd5LUDdZaKtAjdJmMu0dMa1.
        event_type (EventType):
        event_class (EventClass):
        initial_attempt_id (str): The identifier for the initial delivery attempt. This will be the same as `event_id`
            for
            the initial delivery attempt. Example: evt_018e31720d1b7a2b82677d3032cab959.
        created (datetime.datetime): Time at which the event was created. Example: 2022-09-10T10:11:12Z.
        request (OutgoingWebhookRequestContent): The request information (headers and body) sent in the webhook.
        response (OutgoingWebhookResponseContent): The response information (headers, body and status code) received for
            the webhook sent.
        is_delivery_successful (Union[None, Unset, bool]): Indicates whether the webhook was ultimately delivered or
            not.
        delivery_attempt (Union[None, Unset, WebhookDeliveryAttempt]):
    """

    event_id: str
    merchant_id: str
    profile_id: str
    object_id: str
    event_type: EventType
    event_class: EventClass
    initial_attempt_id: str
    created: datetime.datetime
    request: "OutgoingWebhookRequestContent"
    response: "OutgoingWebhookResponseContent"
    is_delivery_successful: Union[None, Unset, bool] = UNSET
    delivery_attempt: Union[None, Unset, WebhookDeliveryAttempt] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        event_id = self.event_id

        merchant_id = self.merchant_id

        profile_id = self.profile_id

        object_id = self.object_id

        event_type = self.event_type.value

        event_class = self.event_class.value

        initial_attempt_id = self.initial_attempt_id

        created = self.created.isoformat()

        request = self.request.to_dict()

        response = self.response.to_dict()

        is_delivery_successful: Union[None, Unset, bool]
        if isinstance(self.is_delivery_successful, Unset):
            is_delivery_successful = UNSET
        else:
            is_delivery_successful = self.is_delivery_successful

        delivery_attempt: Union[None, Unset, str]
        if isinstance(self.delivery_attempt, Unset):
            delivery_attempt = UNSET
        elif isinstance(self.delivery_attempt, WebhookDeliveryAttempt):
            delivery_attempt = self.delivery_attempt.value
        else:
            delivery_attempt = self.delivery_attempt

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "event_id": event_id,
                "merchant_id": merchant_id,
                "profile_id": profile_id,
                "object_id": object_id,
                "event_type": event_type,
                "event_class": event_class,
                "initial_attempt_id": initial_attempt_id,
                "created": created,
                "request": request,
                "response": response,
            }
        )
        if is_delivery_successful is not UNSET:
            field_dict["is_delivery_successful"] = is_delivery_successful
        if delivery_attempt is not UNSET:
            field_dict["delivery_attempt"] = delivery_attempt

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.outgoing_webhook_request_content import OutgoingWebhookRequestContent
        from ..models.outgoing_webhook_response_content import OutgoingWebhookResponseContent

        d = dict(src_dict)
        event_id = d.pop("event_id")

        merchant_id = d.pop("merchant_id")

        profile_id = d.pop("profile_id")

        object_id = d.pop("object_id")

        event_type = EventType(d.pop("event_type"))

        event_class = EventClass(d.pop("event_class"))

        initial_attempt_id = d.pop("initial_attempt_id")

        created = isoparse(d.pop("created"))

        request = OutgoingWebhookRequestContent.from_dict(d.pop("request"))

        response = OutgoingWebhookResponseContent.from_dict(d.pop("response"))

        def _parse_is_delivery_successful(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        is_delivery_successful = _parse_is_delivery_successful(d.pop("is_delivery_successful", UNSET))

        def _parse_delivery_attempt(data: object) -> Union[None, Unset, WebhookDeliveryAttempt]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                delivery_attempt_type_1 = WebhookDeliveryAttempt(data)

                return delivery_attempt_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, WebhookDeliveryAttempt], data)

        delivery_attempt = _parse_delivery_attempt(d.pop("delivery_attempt", UNSET))

        event_retrieve_response = cls(
            event_id=event_id,
            merchant_id=merchant_id,
            profile_id=profile_id,
            object_id=object_id,
            event_type=event_type,
            event_class=event_class,
            initial_attempt_id=initial_attempt_id,
            created=created,
            request=request,
            response=response,
            is_delivery_successful=is_delivery_successful,
            delivery_attempt=delivery_attempt,
        )

        event_retrieve_response.additional_properties = d
        return event_retrieve_response

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
