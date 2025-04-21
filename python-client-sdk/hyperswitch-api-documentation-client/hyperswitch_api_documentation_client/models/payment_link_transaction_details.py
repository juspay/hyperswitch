from collections.abc import Mapping
from typing import TYPE_CHECKING, Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..types import UNSET, Unset

if TYPE_CHECKING:
    from ..models.transaction_details_ui_configuration import TransactionDetailsUiConfiguration


T = TypeVar("T", bound="PaymentLinkTransactionDetails")


@_attrs_define
class PaymentLinkTransactionDetails:
    """
    Attributes:
        key (str): Key for the transaction details Example: Policy-Number.
        value (str): Value for the transaction details Example: 297472368473924.
        ui_configuration (Union['TransactionDetailsUiConfiguration', None, Unset]):
    """

    key: str
    value: str
    ui_configuration: Union["TransactionDetailsUiConfiguration", None, Unset] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        from ..models.transaction_details_ui_configuration import TransactionDetailsUiConfiguration

        key = self.key

        value = self.value

        ui_configuration: Union[None, Unset, dict[str, Any]]
        if isinstance(self.ui_configuration, Unset):
            ui_configuration = UNSET
        elif isinstance(self.ui_configuration, TransactionDetailsUiConfiguration):
            ui_configuration = self.ui_configuration.to_dict()
        else:
            ui_configuration = self.ui_configuration

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "key": key,
                "value": value,
            }
        )
        if ui_configuration is not UNSET:
            field_dict["ui_configuration"] = ui_configuration

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        from ..models.transaction_details_ui_configuration import TransactionDetailsUiConfiguration

        d = dict(src_dict)
        key = d.pop("key")

        value = d.pop("value")

        def _parse_ui_configuration(data: object) -> Union["TransactionDetailsUiConfiguration", None, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, dict):
                    raise TypeError()
                ui_configuration_type_1 = TransactionDetailsUiConfiguration.from_dict(data)

                return ui_configuration_type_1
            except:  # noqa: E722
                pass
            return cast(Union["TransactionDetailsUiConfiguration", None, Unset], data)

        ui_configuration = _parse_ui_configuration(d.pop("ui_configuration", UNSET))

        payment_link_transaction_details = cls(
            key=key,
            value=value,
            ui_configuration=ui_configuration,
        )

        payment_link_transaction_details.additional_properties = d
        return payment_link_transaction_details

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
