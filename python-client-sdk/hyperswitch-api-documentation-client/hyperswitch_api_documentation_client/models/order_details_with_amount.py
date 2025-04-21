from collections.abc import Mapping
from typing import Any, TypeVar, Union, cast

from attrs import define as _attrs_define
from attrs import field as _attrs_field

from ..models.product_type import ProductType
from ..types import UNSET, Unset

T = TypeVar("T", bound="OrderDetailsWithAmount")


@_attrs_define
class OrderDetailsWithAmount:
    """
    Attributes:
        product_name (str): Name of the product that is being purchased Example: shirt.
        quantity (int): The quantity of the product to be purchased Example: 1.
        amount (int): the amount per quantity of product
        tax_rate (Union[None, Unset, float]): tax rate applicable to the product
        total_tax_amount (Union[None, Unset, int]): total tax amount applicable to the product
        requires_shipping (Union[None, Unset, bool]):
        product_img_link (Union[None, Unset, str]): The image URL of the product
        product_id (Union[None, Unset, str]): ID of the product that is being purchased
        category (Union[None, Unset, str]): Category of the product that is being purchased
        sub_category (Union[None, Unset, str]): Sub category of the product that is being purchased
        brand (Union[None, Unset, str]): Brand of the product that is being purchased
        product_type (Union[None, ProductType, Unset]):
        product_tax_code (Union[None, Unset, str]): The tax code for the product
    """

    product_name: str
    quantity: int
    amount: int
    tax_rate: Union[None, Unset, float] = UNSET
    total_tax_amount: Union[None, Unset, int] = UNSET
    requires_shipping: Union[None, Unset, bool] = UNSET
    product_img_link: Union[None, Unset, str] = UNSET
    product_id: Union[None, Unset, str] = UNSET
    category: Union[None, Unset, str] = UNSET
    sub_category: Union[None, Unset, str] = UNSET
    brand: Union[None, Unset, str] = UNSET
    product_type: Union[None, ProductType, Unset] = UNSET
    product_tax_code: Union[None, Unset, str] = UNSET
    additional_properties: dict[str, Any] = _attrs_field(init=False, factory=dict)

    def to_dict(self) -> dict[str, Any]:
        product_name = self.product_name

        quantity = self.quantity

        amount = self.amount

        tax_rate: Union[None, Unset, float]
        if isinstance(self.tax_rate, Unset):
            tax_rate = UNSET
        else:
            tax_rate = self.tax_rate

        total_tax_amount: Union[None, Unset, int]
        if isinstance(self.total_tax_amount, Unset):
            total_tax_amount = UNSET
        else:
            total_tax_amount = self.total_tax_amount

        requires_shipping: Union[None, Unset, bool]
        if isinstance(self.requires_shipping, Unset):
            requires_shipping = UNSET
        else:
            requires_shipping = self.requires_shipping

        product_img_link: Union[None, Unset, str]
        if isinstance(self.product_img_link, Unset):
            product_img_link = UNSET
        else:
            product_img_link = self.product_img_link

        product_id: Union[None, Unset, str]
        if isinstance(self.product_id, Unset):
            product_id = UNSET
        else:
            product_id = self.product_id

        category: Union[None, Unset, str]
        if isinstance(self.category, Unset):
            category = UNSET
        else:
            category = self.category

        sub_category: Union[None, Unset, str]
        if isinstance(self.sub_category, Unset):
            sub_category = UNSET
        else:
            sub_category = self.sub_category

        brand: Union[None, Unset, str]
        if isinstance(self.brand, Unset):
            brand = UNSET
        else:
            brand = self.brand

        product_type: Union[None, Unset, str]
        if isinstance(self.product_type, Unset):
            product_type = UNSET
        elif isinstance(self.product_type, ProductType):
            product_type = self.product_type.value
        else:
            product_type = self.product_type

        product_tax_code: Union[None, Unset, str]
        if isinstance(self.product_tax_code, Unset):
            product_tax_code = UNSET
        else:
            product_tax_code = self.product_tax_code

        field_dict: dict[str, Any] = {}
        field_dict.update(self.additional_properties)
        field_dict.update(
            {
                "product_name": product_name,
                "quantity": quantity,
                "amount": amount,
            }
        )
        if tax_rate is not UNSET:
            field_dict["tax_rate"] = tax_rate
        if total_tax_amount is not UNSET:
            field_dict["total_tax_amount"] = total_tax_amount
        if requires_shipping is not UNSET:
            field_dict["requires_shipping"] = requires_shipping
        if product_img_link is not UNSET:
            field_dict["product_img_link"] = product_img_link
        if product_id is not UNSET:
            field_dict["product_id"] = product_id
        if category is not UNSET:
            field_dict["category"] = category
        if sub_category is not UNSET:
            field_dict["sub_category"] = sub_category
        if brand is not UNSET:
            field_dict["brand"] = brand
        if product_type is not UNSET:
            field_dict["product_type"] = product_type
        if product_tax_code is not UNSET:
            field_dict["product_tax_code"] = product_tax_code

        return field_dict

    @classmethod
    def from_dict(cls: type[T], src_dict: Mapping[str, Any]) -> T:
        d = dict(src_dict)
        product_name = d.pop("product_name")

        quantity = d.pop("quantity")

        amount = d.pop("amount")

        def _parse_tax_rate(data: object) -> Union[None, Unset, float]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, float], data)

        tax_rate = _parse_tax_rate(d.pop("tax_rate", UNSET))

        def _parse_total_tax_amount(data: object) -> Union[None, Unset, int]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, int], data)

        total_tax_amount = _parse_total_tax_amount(d.pop("total_tax_amount", UNSET))

        def _parse_requires_shipping(data: object) -> Union[None, Unset, bool]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, bool], data)

        requires_shipping = _parse_requires_shipping(d.pop("requires_shipping", UNSET))

        def _parse_product_img_link(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        product_img_link = _parse_product_img_link(d.pop("product_img_link", UNSET))

        def _parse_product_id(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        product_id = _parse_product_id(d.pop("product_id", UNSET))

        def _parse_category(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        category = _parse_category(d.pop("category", UNSET))

        def _parse_sub_category(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        sub_category = _parse_sub_category(d.pop("sub_category", UNSET))

        def _parse_brand(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        brand = _parse_brand(d.pop("brand", UNSET))

        def _parse_product_type(data: object) -> Union[None, ProductType, Unset]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            try:
                if not isinstance(data, str):
                    raise TypeError()
                product_type_type_1 = ProductType(data)

                return product_type_type_1
            except:  # noqa: E722
                pass
            return cast(Union[None, ProductType, Unset], data)

        product_type = _parse_product_type(d.pop("product_type", UNSET))

        def _parse_product_tax_code(data: object) -> Union[None, Unset, str]:
            if data is None:
                return data
            if isinstance(data, Unset):
                return data
            return cast(Union[None, Unset, str], data)

        product_tax_code = _parse_product_tax_code(d.pop("product_tax_code", UNSET))

        order_details_with_amount = cls(
            product_name=product_name,
            quantity=quantity,
            amount=amount,
            tax_rate=tax_rate,
            total_tax_amount=total_tax_amount,
            requires_shipping=requires_shipping,
            product_img_link=product_img_link,
            product_id=product_id,
            category=category,
            sub_category=sub_category,
            brand=brand,
            product_type=product_type,
            product_tax_code=product_tax_code,
        )

        order_details_with_amount.additional_properties = d
        return order_details_with_amount

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
