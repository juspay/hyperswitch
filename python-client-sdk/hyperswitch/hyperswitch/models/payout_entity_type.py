from enum import Enum


class PayoutEntityType(str, Enum):
    COMPANY = "Company"
    INDIVIDUAL = "Individual"
    LOWERCASE = "lowercase"
    NATURALPERSON = "NaturalPerson"
    NONPROFIT = "NonProfit"
    PERSONAL = "Personal"
    PUBLICSECTOR = "PublicSector"

    def __str__(self) -> str:
        return str(self.value)
