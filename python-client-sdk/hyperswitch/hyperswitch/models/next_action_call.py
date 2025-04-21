from enum import Enum


class NextActionCall(str, Enum):
    COMPLETE_AUTHORIZE = "complete_authorize"
    CONFIRM = "confirm"
    POST_SESSION_TOKENS = "post_session_tokens"
    SYNC = "sync"

    def __str__(self) -> str:
        return str(self.value)
