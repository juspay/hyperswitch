from enum import Enum


class FieldTypeType44(str, Enum):
    ORDER_DETAILS_PRODUCT_NAME = "order_details_product_name"

    def __str__(self) -> str:
        return str(self.value)
