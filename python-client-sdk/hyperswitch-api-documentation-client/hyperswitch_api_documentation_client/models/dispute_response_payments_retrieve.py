import datetime
from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.dispute_stage import DisputeStage
from ..models.dispute_status import DisputeStatus
from ..types import UNSET, Unset

T = TypeVar("T", bound="DisputeResponsePaymentsRetrieve")


@_attrs_define
class DisputeResponsePaymentsRetrieve:
    """
    Attributes:
        dispute_id (str): The identifier for dispute
        dispute_stage (DisputeStage): Stage of the dispute
        dispute_status (DisputeStatus): Status of the dispute
        connector_status (str): Status of the dispute sent by connector
        connector_dispute_id (str): Dispute id sent by connector
        created_at (datetime.datetime): Time at which dispute is received
        connector_reason (Union[None, Unset, str]): Reason of dispute sent by connector
        connector_reason_code (Union[None, Unset, str]): Reason code of dispute sent by connector
        challenge_required_by (Union[None, Unset, datetime.datetime]): Evidence deadline of dispute sent by connector
        connector_created_at (Union[None, Unset, datetime.datetime]): Dispute created time sent by connector
        connector_updated_at (Union[None, Unset, datetime.datetime]): Dispute updated time sent by connector
    """

    dispute_id: str
    dispute_stage: DisputeStage
    dispute_status: DisputeStatus
    connector_status: str
    connector_dispute_id: str
    created_at: datetime.datetime
    connector_reason: Union[None, Unset, str] = UNSET
    connector_reason_code: Union[None, Unset, str] = UNSET
    challenge_required_by: Union[None, Unset, datetime.datetime] = UNSET
    connector_created_at: Union[None, Unset, datetime.datetime] = UNSET
    connector_updated_at: Union[None, Unset, datetime.datetime] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        dispute_id = self.dispute_id

        dispute_stage = self.dispute_stage.value

        dispute_status = self.dispute_status.value

        connector_status = self.connector_status

        connector_dispute_id = self.connector_dispute_id

        created_at = self.created_at.isoformat()

        connector_reason: Union[None, Unset, str]
        if isinstance(self.connector_reason, Unset):
            connector_reason = UNSET
        else:
            connector_reason = self.connector_reason

        connector_reason_code: Union[None, Unset, str]
        if isinstance(self.connector_reason_code, Unset):
            connector_reason_code = UNSET
        else:
            connector_reason_code = self.connector_reason_code

        challenge_required_by: Union[None, Unset, str]
        if isinstance(self.challenge_required_by, Unset):
            challenge_required_by = UNSET
        elif isinstance(self.challenge_required_by, datetime.datetime):
            challenge_required_by = self.challenge_required_by.isoformat()
        else:
            challenge_required_by = self.challenge_required_by

        connector_created_at: Union[None, Unset, str]
        if isinstance(self.connector_created_at, Unset):
            connector_created_at = UNSET
        elif isinstance(self.connector_created_at, datetime.datetime):
            connector_created_at = self.connector_created_at.isoformat()
        else:
            connector_created_at = self.connector_created_at

        connector_updated_at: Union[None, Unset, str]
        if isinstance(self.connector_updated_at, Unset):
            connector_updated_at = UNSET
        elif isinstance(self.connector_updated_at, datetime.datetime):
            connector_updated_at = self.connector_updated_at.isoformat()
        else:
            connector_updated_at = self.connector_updated_at

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "dispute_id": dispute_id,
                "dispute_stage": dispute_stage,
                "dispute_status": dispute_status,
                "connector_status": connector_status,
                "connector_dispute_id": connector_dispute_id,
                "created_at": created_at,
            }
        )
        if connector_reason is not UNSET:
            field_dict["connector_reason"] = connector_reason
        if connector_reason_code is not UNSET:
            field_dict["connector_reason_code"] = connector_reason_code
        if challenge_required_by is not UNSET:
            field_dict["challenge_required_by"] = challenge_required_by
        if connector_created_at is not UNSET:
            field_dict["connector_created_at"] = connector_created_at
        if connector_updated_at is not UNSET:
            field_dict["connector_updated_at"] = connector_updated_at

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        dispute_id = d.pop("dispute_id")

        dispute_stage = DisputeStage(d.pop("dispute_stage"))

        dispute_status = DisputeStatus(d.pop("dispute_status"))

        connector_status = d.pop("connector_status")

        connector_dispute_id = d.pop("connector_dispute_id")

        created_at = isoparse(d.pop("created_at"))

        def _parse_connector_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_reason = _parse_connector_reason(d.pop("connector_reason", UNSET))

        def _parse_connector_reason_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        connector_reason_code = _parse_connector_reason_code(d.pop("connector_reason_code", UNSET))

        def _parse_challenge_required_by(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                challenge_required_by_type_0 = isoparse(data)

                return challenge_required_by_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        challenge_required_by = _parse_challenge_required_by(d.pop("challenge_required_by", UNSET))

        def _parse_connector_created_at(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                connector_created_at_type_0 = isoparse(data)

                return connector_created_at_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        connector_created_at = _parse_connector_created_at(d.pop("connector_created_at", UNSET))

        def _parse_connector_updated_at(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                connector_updated_at_type_0 = isoparse(data)

                return connector_updated_at_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        connector_updated_at = _parse_connector_updated_at(d.pop("connector_updated_at", UNSET))

        dispute_response_payments_retrieve = cls(
            dispute_id=dispute_id,
            dispute_stage=dispute_stage,
            dispute_status=dispute_status,
            connector_status=connector_status,
            connector_dispute_id=connector_dispute_id,
            created_at=created_at,
            connector_reason=connector_reason,
            connector_reason_code=connector_reason_code,
            challenge_required_by=challenge_required_by,
            connector_created_at=connector_created_at,
            connector_updated_at=connector_updated_at,
        )

        dispute_response_payments_retrieve.additional_properties = d
        return dispute_response_payments_retrieve

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
