from enum import Enum


class ElementPosition(str, Enum):
    BOTTOM = "bottom"
    BOTTOM_LEFT = "bottom left"
    BOTTOM_RIGHT = "bottom right"
    CENTER = "center"
    LEFT = "left"
    RIGHT = "right"
    TOP = "top"
    TOP_LEFT = "top left"
    TOP_RIGHT = "top right"

    def __str__(self) -> str:
        return str(self.value)
