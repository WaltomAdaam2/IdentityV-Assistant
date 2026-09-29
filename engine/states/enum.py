from enum import Enum


class GameState(str, Enum):
    UNKNOWN = "unknown"
    LOBBY = "lobby"
    MODE_SELECT = "mode_select"
    MATCHMAKING = "matchmaking"
    LOADING = "loading"
    IN_MATCH = "in_match"
    RESULT = "result"
    POPUP = "popup"
