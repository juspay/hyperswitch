from enum import Enum


class UIWidgetFormLayout(str, Enum):
    JOURNEY = "journey"
    TABS = "tabs"

    def __str__(self) -> str:
        return str(self.value)
