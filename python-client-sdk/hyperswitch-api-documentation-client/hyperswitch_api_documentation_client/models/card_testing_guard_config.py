from collections.abc import Mapping
from typing import Any, TypeVar

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.card_testing_guard_status import CardTestingGuardStatus

T = TypeVar("T", bound="CardTestingGuardConfig")


@_attrs_define
class CardTestingGuardConfig:
    """
    Attributes:
        card_ip_blocking_status (CardTestingGuardStatus):
        card_ip_blocking_threshold (int): Determines the unsuccessful payment threshold for Card IP Blocking for profile
        guest_user_card_blocking_status (CardTestingGuardStatus):
        guest_user_card_blocking_threshold (int): Determines the unsuccessful payment threshold for Guest User Card
            Blocking for profile
        customer_id_blocking_status (CardTestingGuardStatus):
        customer_id_blocking_threshold (int): Determines the unsuccessful payment threshold for Customer Id Blocking for
            profile
        card_testing_guard_expiry (int): Determines Redis Expiry for Card Testing Guard for profile
    """

    card_ip_blocking_status: CardTestingGuardStatus
    card_ip_blocking_threshold: int
    guest_user_card_blocking_status: CardTestingGuardStatus
    guest_user_card_blocking_threshold: int
    customer_id_blocking_status: CardTestingGuardStatus
    customer_id_blocking_threshold: int
    card_testing_guard_expiry: int
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        card_ip_blocking_status = self.card_ip_blocking_status.value

        card_ip_blocking_threshold = self.card_ip_blocking_threshold

        guest_user_card_blocking_status = self.guest_user_card_blocking_status.value

        guest_user_card_blocking_threshold = self.guest_user_card_blocking_threshold

        customer_id_blocking_status = self.customer_id_blocking_status.value

        customer_id_blocking_threshold = self.customer_id_blocking_threshold

        card_testing_guard_expiry = self.card_testing_guard_expiry

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "card_ip_blocking_status": card_ip_blocking_status,
                "card_ip_blocking_threshold": card_ip_blocking_threshold,
                "guest_user_card_blocking_status": guest_user_card_blocking_status,
                "guest_user_card_blocking_threshold": guest_user_card_blocking_threshold,
                "customer_id_blocking_status": customer_id_blocking_status,
                "customer_id_blocking_threshold": customer_id_blocking_threshold,
                "card_testing_guard_expiry": card_testing_guard_expiry,
            }
        )

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        card_ip_blocking_status = CardTestingGuardStatus(d.pop("card_ip_blocking_status"))

        card_ip_blocking_threshold = d.pop("card_ip_blocking_threshold")

        guest_user_card_blocking_status = CardTestingGuardStatus(d.pop("guest_user_card_blocking_status"))

        guest_user_card_blocking_threshold = d.pop("guest_user_card_blocking_threshold")

        customer_id_blocking_status = CardTestingGuardStatus(d.pop("customer_id_blocking_status"))

        customer_id_blocking_threshold = d.pop("customer_id_blocking_threshold")

        card_testing_guard_expiry = d.pop("card_testing_guard_expiry")

        card_testing_guard_config = cls(
            card_ip_blocking_status=card_ip_blocking_status,
            card_ip_blocking_threshold=card_ip_blocking_threshold,
            guest_user_card_blocking_status=guest_user_card_blocking_status,
            guest_user_card_blocking_threshold=guest_user_card_blocking_threshold,
            customer_id_blocking_status=customer_id_blocking_status,
            customer_id_blocking_threshold=customer_id_blocking_threshold,
            card_testing_guard_expiry=card_testing_guard_expiry,
        )

        card_testing_guard_config.additional_properties = d
        return card_testing_guard_config

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
