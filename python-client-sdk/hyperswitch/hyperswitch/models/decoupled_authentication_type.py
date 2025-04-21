from enum import Enum


class DecoupledAuthenticationType(str, Enum):
    CHALLENGE = "challenge"
    FRICTIONLESS = "frictionless"

    def __str__(self) -> str:
        return str(self.value)
