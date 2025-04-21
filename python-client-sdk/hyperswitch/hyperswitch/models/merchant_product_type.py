from enum import Enum


class MerchantProductType(str, Enum):
    COST_OBSERVABILITY = "cost_observability"
    DYNAMIC_ROUTING = "dynamic_routing"
    ORCHESTRATION = "orchestration"
    RECON = "recon"
    RECOVERY = "recovery"
    VAULT = "vault"

    def __str__(self) -> str:
        return str(self.value)
