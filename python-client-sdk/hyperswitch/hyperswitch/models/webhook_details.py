from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define

from ..types import UNSET, Unset

T = TypeVar("T", bound="WebhookDetails")


@_attrs_define
class WebhookDetails:
    """
    Attributes:
        webhook_version (Union[None, Unset, str]): The version for Webhook Example: 1.0.2.
        webhook_username (Union[None, Unset, str]): The user name for Webhook login Example: ekart_retail.
        webhook_password (Union[None, Unset, str]): The password for Webhook login Example: ekart@123.
        webhook_url (Union[None, Unset, str]): The url for the webhook endpoint Example: www.ekart.com/webhooks.
        payment_created_enabled (Union[None, Unset, bool]): If this property is true, a webhook message is posted
            whenever a new payment is created Example: True.
        payment_succeeded_enabled (Union[None, Unset, bool]): If this property is true, a webhook message is posted
            whenever a payment is successful Example: True.
        payment_failed_enabled (Union[None, Unset, bool]): If this property is true, a webhook message is posted
            whenever a payment fails Example: True.
    """

    webhook_version: Union[None, Unset, str] = UNSET
    webhook_username: Union[None, Unset, str] = UNSET
    webhook_password: Union[None, Unset, str] = UNSET
    webhook_url: Union[None, Unset, str] = UNSET
    payment_created_enabled: Union[None, Unset, bool] = UNSET
    payment_succeeded_enabled: Union[None, Unset, bool] = UNSET
    payment_failed_enabled: Union[None, Unset, bool] = UNSET

    def to_dict(self) -> dict[str, Any]:
        webhook_version: Union[None, Unset, str]
        if isinstance(self.webhook_version, Unset):
            webhook_version = UNSET
        else:
            webhook_version = self.webhook_version

        webhook_username: Union[None, Unset, str]
        if isinstance(self.webhook_username, Unset):
            webhook_username = UNSET
        else:
            webhook_username = self.webhook_username

        webhook_password: Union[None, Unset, str]
        if isinstance(self.webhook_password, Unset):
            webhook_password = UNSET
        else:
            webhook_password = self.webhook_password

        webhook_url: Union[None, Unset, str]
        if isinstance(self.webhook_url, Unset):
            webhook_url = UNSET
        else:
            webhook_url = self.webhook_url

        payment_created_enabled: Union[None, Unset, bool]
        if isinstance(self.payment_created_enabled, Unset):
            payment_created_enabled = UNSET
        else:
            payment_created_enabled = self.payment_created_enabled

        payment_succeeded_enabled: Union[None, Unset, bool]
        if isinstance(self.payment_succeeded_enabled, Unset):
            payment_succeeded_enabled = UNSET
        else:
            payment_succeeded_enabled = self.payment_succeeded_enabled

        payment_failed_enabled: Union[None, Unset, bool]
        if isinstance(self.payment_failed_enabled, Unset):
            payment_failed_enabled = UNSET
        else:
            payment_failed_enabled = self.payment_failed_enabled

        field_dict: dict[str, Any] = {}
        field_dict.update({})
        if webhook_version is not UNSET:
            field_dict["webhook_version"] = webhook_version
        if webhook_username is not UNSET:
            field_dict["webhook_username"] = webhook_username
        if webhook_password is not UNSET:
            field_dict["webhook_password"] = webhook_password
        if webhook_url is not UNSET:
            field_dict["webhook_url"] = webhook_url
        if payment_created_enabled is not UNSET:
            field_dict["payment_created_enabled"] = payment_created_enabled
        if payment_succeeded_enabled is not UNSET:
            field_dict["payment_succeeded_enabled"] = payment_succeeded_enabled
        if payment_failed_enabled is not UNSET:
            field_dict["payment_failed_enabled"] = payment_failed_enabled

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)

        def _parse_webhook_version(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        webhook_version = _parse_webhook_version(d.pop("webhook_version", UNSET))

        def _parse_webhook_username(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        webhook_username = _parse_webhook_username(d.pop("webhook_username", UNSET))

        def _parse_webhook_password(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        webhook_password = _parse_webhook_password(d.pop("webhook_password", UNSET))

        def _parse_webhook_url(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        webhook_url = _parse_webhook_url(d.pop("webhook_url", UNSET))

        def _parse_payment_created_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        payment_created_enabled = _parse_payment_created_enabled(d.pop("payment_created_enabled", UNSET))

        def _parse_payment_succeeded_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        payment_succeeded_enabled = _parse_payment_succeeded_enabled(d.pop("payment_succeeded_enabled", UNSET))

        def _parse_payment_failed_enabled(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        payment_failed_enabled = _parse_payment_failed_enabled(d.pop("payment_failed_enabled", UNSET))

        webhook_details = cls(
            webhook_version=webhook_version,
            webhook_username=webhook_username,
            webhook_password=webhook_password,
            webhook_url=webhook_url,
            payment_created_enabled=payment_created_enabled,
            payment_succeeded_enabled=payment_succeeded_enabled,
            payment_failed_enabled=payment_failed_enabled,
        )

        return webhook_details
