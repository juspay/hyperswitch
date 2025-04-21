from enum import Enum


class DynamicRoutingFeatures(str, Enum):
    DYNAMIC_CONNECTOR_SELECTION = "dynamic_connector_selection"
    METRICS = "metrics"
    NONE = "none"

    def __str__(self) -> str:
        return str(self.value)
