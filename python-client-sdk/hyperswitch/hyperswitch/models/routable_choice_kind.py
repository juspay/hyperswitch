from enum import Enum


class RoutableChoiceKind(str, Enum):
    FULLSTRUCT = "FullStruct"
    ONLYCONNECTOR = "OnlyConnector"

    def __str__(self) -> str:
        return str(self.value)
