# State machine

The game state model currently defines `unknown`, `game_not_found`, `startup`, `login`, `lobby`, `mode_select`, `matchmaking`, `loading`, `in_match`, `result`, `reward`, `weekly_limit`, `network_error`, and `popup`.

The mock task follows:

```text
UNKNOWN -> LOBBY -> MODE_SELECT -> MATCHMAKING -> LOADING -> IN_MATCH -> RESULT -> LOBBY
```

Real detectors produce a `StateDetection` containing a state, confidence, and evidence list. `StateHistory` confirms ordinary transitions across multiple observations so one uncertain frame does not cause state churn. `UNKNOWN` is retried and measured; it is a valid observation, not an exception.

The weekly task is intentionally a placeholder. Its handlers return semantic planned actions without guessed screen coordinates.
