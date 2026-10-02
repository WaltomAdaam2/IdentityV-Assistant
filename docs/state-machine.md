# State machine

The game state model currently defines `unknown`, `game_not_found`, `startup`, `login`, `lobby`, `mode_select`, `matchmaking`, `loading`, `in_match`, `result`, `reward`, `weekly_limit`, `network_error`, and `popup`.

The Blackjack task follows:

```text
HOME -> BOOK -> CHAOS -> BLACKJACK -> MATCHMAKING -> READY -> CARD -> IN_MATCH -> RESULT -> HOME
```

The Blackjack detector normalizes frames to 1920 × 1080, searches small template regions, and confirms a critical screen in two observations at least 10 ms apart. Home detection uses ten sampled pixels and requires eight matches. After a match, the task confirms the result and return buttons, then counts a completed cycle only after home is recognized again.

## Hide and Seek

Hide and Seek uses the following routing path. A task may enter at any recognized stage; it only performs the remaining actions needed to return home.

```text
HOME -> BOOK -> CHAOS -> HIDE_AND_SEEK -> BAD_CHILD + START_GAME
     -> MATCHING -> READY_CASE -> READY_START
     -> F_INITIAL -> R_BLOCKED -> V_READY -> V_FIXED
     -> DEATH_F -> CONTINUE -> RETURN_LOBBY -> HOME
```

`F_INITIAL` moves for 1–3 seconds and presses F. At `V_READY`, the task confirms the R icon with a red X, moves for 5–8 seconds, and presses V. At `V_FIXED`, it waits 5 seconds, then runs independent randomized movement and camera-turn loops. Movement holds one WASD key for 13–17 seconds, followed by a 1–5 second gap. Small camera turns last 2.5–3.5 seconds with a 2.5–3.5 second cooldown. A stage change, pause, stop, or exit from a recognized in-round stage releases held input; resume checks the current screen before sending more input.

`MATCHING`, `READY_CASE`, `READY_START`, `DEATH_F`, `CONTINUE`, and `RETURN_LOBBY` are known waits and may remain on screen indefinitely. Unknown frames and continuous loading still time out after 300 active seconds. Active time excludes paused time: pause releases held input and freezes every task deadline; resume captures and reroutes the screen before more input is sent.

Template regions are defined under `assets/templates/hide_seek`; the detector normalizes frames to 1920 × 1080 and confirms recognized stages with a second observation. The supplied reference frames and unit tests cover detector routing and stage transitions. No live Identity V game session was used to validate these actions.
