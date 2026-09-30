# State machine

The game state model currently defines `unknown`, `game_not_found`, `startup`, `login`, `lobby`, `mode_select`, `matchmaking`, `loading`, `in_match`, `result`, `reward`, `weekly_limit`, `network_error`, and `popup`.

The Blackjack task follows:

```text
HOME -> BOOK -> CHAOS -> BLACKJACK -> MATCHMAKING -> READY -> CARD -> IN_MATCH -> RESULT -> HOME
```

The Blackjack detector normalizes frames to 1920 × 1080, searches small template regions, and confirms a critical screen in two observations at least 10 ms apart. Home detection uses ten sampled pixels and requires eight matches. Loading pauses each ordinary 300-second stage timeout, while 300 seconds of continuous loading is an error. After a match, the task confirms the result and return buttons, then counts a completed cycle only after home is recognized again.
