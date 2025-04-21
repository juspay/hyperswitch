from enum import Enum


class FrmPreferredFlowTypes(str, Enum):
    POST = "post"
    PRE = "pre"

    def __str__(self) -> str:
        return str(self.value)
