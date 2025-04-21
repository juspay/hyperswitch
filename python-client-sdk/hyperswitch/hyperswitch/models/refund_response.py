import datetime
from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field
from dateutil.parser import isoparse

from ..models.refund_status import RefundStatus
from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.refund_response_metadata_type_0 import RefundResponseMetadataType0
    from ..models.split_refund_type_0 import SplitRefundType0
    from ..models.split_refund_type_1 import SplitRefundType1
    from ..models.split_refund_type_2 import SplitRefundType2


T = TypeVar("T", bound="RefundResponse")


@_attrs_define
class RefundResponse:
    """
    Attributes:
        refund_id (str): Unique Identifier for the refund
        payment_id (str): The payment id against which refund is initiated
        amount (int): The refund amount, which should be less than or equal to the total payment amount. Amount for the
            payment in lowest denomination of the currency. (i.e) in cents for USD denomination, in paisa for INR
            denomination etc Example: 6540.
        currency (str): The three-letter ISO currency code
        status (RefundStatus): The status for refunds
        connector (str): The connector used for the refund and the corresponding payment Example: stripe.
        reason (Union[None, Unset, str]): An arbitrary string attached to the object. Often useful for displaying to
            users and your customer support executive
        metadata (Union['RefundResponseMetadataType0', None, Unset]): You can specify up to 50 keys, with key names up
            to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional,
            structured information on an object
        error_message (Union[None, Unset, str]): The error message
        error_code (Union[None, Unset, str]): The code for the error
        unified_code (Union[None, Unset, str]): Error code unified across the connectors is received here if there was
            an error while calling connector
        unified_message (Union[None, Unset, str]): Error message unified across the connectors is received here if there
            was an error while calling connector
        created_at (Union[None, Unset, datetime.datetime]): The timestamp at which refund is created
        updated_at (Union[None, Unset, datetime.datetime]): The timestamp at which refund is updated
        profile_id (Union[None, Unset, str]): The id of business profile for this refund
        merchant_connector_id (Union[None, Unset, str]): The merchant_connector_id of the processor through which this
            payment went through
        split_refunds (Union['SplitRefundType0', 'SplitRefundType1', 'SplitRefundType2', None, Unset]):
        issuer_error_code (Union[None, Unset, str]): Error code received from the issuer in case of failed refunds
        issuer_error_message (Union[None, Unset, str]): Error message received from the issuer in case of failed refunds
    """

    refund_id: str
    payment_id: str
    amount: int
    currency: str
    status: RefundStatus
    connector: str
    reason: Union[None, Unset, str] = UNSET
    metadata: Union["RefundResponseMetadataType0", None, Unset] = UNSET
    error_message: Union[None, Unset, str] = UNSET
    error_code: Union[None, Unset, str] = UNSET
    unified_code: Union[None, Unset, str] = UNSET
    unified_message: Union[None, Unset, str] = UNSET
    created_at: Union[None, Unset, datetime.datetime] = UNSET
    updated_at: Union[None, Unset, datetime.datetime] = UNSET
    profile_id: Union[None, Unset, str] = UNSET
    merchant_connector_id: Union[None, Unset, str] = UNSET
    split_refunds: Union["SplitRefundType0", "SplitRefundType1", "SplitRefundType2", None, Unset] = UNSET
    issuer_error_code: Union[None, Unset, str] = UNSET
    issuer_error_message: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.refund_response_metadata_type_0 import RefundResponseMetadataType0
        from ..models.split_refund_type_0 import SplitRefundType0
        from ..models.split_refund_type_1 import SplitRefundType1
        from ..models.split_refund_type_2 import SplitRefundType2

        refund_id = self.refund_id

        payment_id = self.payment_id

        amount = self.amount

        currency = self.currency

        status = self.status.value

        connector = self.connector

        reason: Union[None, Unset, str]
        if isinstance(self.reason, Unset):
            reason = UNSET
        else:
            reason = self.reason

        metadata: Union[None, Unset, dict[str, Any]]
        if isinstance(self.metadata, Unset):
            metadata = UNSET
        elif isinstance(self.metadata, RefundResponseMetadataType0):
            metadata = self.metadata.to_dict()
        else:
            metadata = self.metadata

        error_message: Union[None, Unset, str]
        if isinstance(self.error_message, Unset):
            error_message = UNSET
        else:
            error_message = self.error_message

        error_code: Union[None, Unset, str]
        if isinstance(self.error_code, Unset):
            error_code = UNSET
        else:
            error_code = self.error_code

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

        created_at: Union[None, Unset, str]
        if isinstance(self.created_at, Unset):
            created_at = UNSET
        elif isinstance(self.created_at, datetime.datetime):
            created_at = self.created_at.isoformat()
        else:
            created_at = self.created_at

        updated_at: Union[None, Unset, str]
        if isinstance(self.updated_at, Unset):
            updated_at = UNSET
        elif isinstance(self.updated_at, datetime.datetime):
            updated_at = self.updated_at.isoformat()
        else:
            updated_at = self.updated_at

        profile_id: Union[None, Unset, str]
        if isinstance(self.profile_id, Unset):
            profile_id = UNSET
        else:
            profile_id = self.profile_id

        merchant_connector_id: Union[None, Unset, str]
        if isinstance(self.merchant_connector_id, Unset):
            merchant_connector_id = UNSET
        else:
            merchant_connector_id = self.merchant_connector_id

        split_refunds: Union[None, Unset, dict[str, Any]]
        if isinstance(self.split_refunds, Unset):
            split_refunds = UNSET
        elif isinstance(self.split_refunds, SplitRefundType0):
            split_refunds = self.split_refunds.to_dict()
        elif isinstance(self.split_refunds, SplitRefundType1):
            split_refunds = self.split_refunds.to_dict()
        elif isinstance(self.split_refunds, SplitRefundType2):
            split_refunds = self.split_refunds.to_dict()
        else:
            split_refunds = self.split_refunds

        issuer_error_code: Union[None, Unset, str]
        if isinstance(self.issuer_error_code, Unset):
            issuer_error_code = UNSET
        else:
            issuer_error_code = self.issuer_error_code

        issuer_error_message: Union[None, Unset, str]
        if isinstance(self.issuer_error_message, Unset):
            issuer_error_message = UNSET
        else:
            issuer_error_message = self.issuer_error_message

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "refund_id": refund_id,
                "payment_id": payment_id,
                "amount": amount,
                "currency": currency,
                "status": status,
                "connector": connector,
            }
        )
        if reason is not UNSET:
            field_dict["reason"] = reason
        if metadata is not UNSET:
            field_dict["metadata"] = metadata
        if error_message is not UNSET:
            field_dict["error_message"] = error_message
        if error_code is not UNSET:
            field_dict["error_code"] = error_code
        if unified_code is not UNSET:
            field_dict["unified_code"] = unified_code
        if unified_message is not UNSET:
            field_dict["unified_message"] = unified_message
        if created_at is not UNSET:
            field_dict["created_at"] = created_at
        if updated_at is not UNSET:
            field_dict["updated_at"] = updated_at
        if profile_id is not UNSET:
            field_dict["profile_id"] = profile_id
        if merchant_connector_id is not UNSET:
            field_dict["merchant_connector_id"] = merchant_connector_id
        if split_refunds is not UNSET:
            field_dict["split_refunds"] = split_refunds
        if issuer_error_code is not UNSET:
            field_dict["issuer_error_code"] = issuer_error_code
        if issuer_error_message is not UNSET:
            field_dict["issuer_error_message"] = issuer_error_message

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.refund_response_metadata_type_0 import RefundResponseMetadataType0
        from ..models.split_refund_type_0 import SplitRefundType0
        from ..models.split_refund_type_1 import SplitRefundType1
        from ..models.split_refund_type_2 import SplitRefundType2

        d = dict(src_dict)
        refund_id = d.pop("refund_id")

        payment_id = d.pop("payment_id")

        amount = d.pop("amount")

        currency = d.pop("currency")

        status = RefundStatus(d.pop("status"))

        connector = d.pop("connector")

        def _parse_reason(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        reason = _parse_reason(d.pop("reason", UNSET))

        def _parse_metadata(data: object) -> Union["RefundResponseMetadataType0", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                metadata_type_0 = RefundResponseMetadataType0.from_dict(data)

                return metadata_type_0
            except:  # noqa: E722
                pass
            return cast(Union["RefundResponseMetadataType0", None, Unset], data)

        metadata = _parse_metadata(d.pop("metadata", UNSET))

        def _parse_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_message = _parse_error_message(d.pop("error_message", UNSET))

        def _parse_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        error_code = _parse_error_code(d.pop("error_code", UNSET))

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

        def _parse_created_at(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                created_at_type_0 = isoparse(data)

                return created_at_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        created_at = _parse_created_at(d.pop("created_at", UNSET))

        def _parse_updated_at(data: object) -> Union[None, Unset, datetime.datetime]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                updated_at_type_0 = isoparse(data)

                return updated_at_type_0
            except:  # noqa: E722
                pass
            return cast(Union[None, Unset, datetime.datetime], data)

        updated_at = _parse_updated_at(d.pop("updated_at", UNSET))

        def _parse_profile_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        profile_id = _parse_profile_id(d.pop("profile_id", UNSET))

        def _parse_merchant_connector_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        merchant_connector_id = _parse_merchant_connector_id(d.pop("merchant_connector_id", UNSET))

        def _parse_split_refunds(
            data: object,
        ) -> Union["SplitRefundType0", "SplitRefundType1", "SplitRefundType2", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_split_refund_type_0 = SplitRefundType0.from_dict(data)

                return componentsschemas_split_refund_type_0
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_split_refund_type_1 = SplitRefundType1.from_dict(data)

                return componentsschemas_split_refund_type_1
            except:  # noqa: E722
                pass
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                componentsschemas_split_refund_type_2 = SplitRefundType2.from_dict(data)

                return componentsschemas_split_refund_type_2
            except:  # noqa: E722
                pass
            return cast(Union["SplitRefundType0", "SplitRefundType1", "SplitRefundType2", None, Unset], data)

        split_refunds = _parse_split_refunds(d.pop("split_refunds", UNSET))

        def _parse_issuer_error_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        issuer_error_code = _parse_issuer_error_code(d.pop("issuer_error_code", UNSET))

        def _parse_issuer_error_message(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        issuer_error_message = _parse_issuer_error_message(d.pop("issuer_error_message", UNSET))

        refund_response = cls(
            refund_id=refund_id,
            payment_id=payment_id,
            amount=amount,
            currency=currency,
            status=status,
            connector=connector,
            reason=reason,
            metadata=metadata,
            error_message=error_message,
            error_code=error_code,
            unified_code=unified_code,
            unified_message=unified_message,
            created_at=created_at,
            updated_at=updated_at,
            profile_id=profile_id,
            merchant_connector_id=merchant_connector_id,
            split_refunds=split_refunds,
            issuer_error_code=issuer_error_code,
            issuer_error_message=issuer_error_message,
        )

        refund_response.additional_properties = d
        return refund_response

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
