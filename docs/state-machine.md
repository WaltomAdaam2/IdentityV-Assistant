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

`F_INITIAL` moves for 1–3 seconds and presses F. At `V_READY`, the task confirms the R icon with a red X, moves for 5–8 seconds, and presses V. At `V_FIXED`, camera turns begin immediately, while movement starts after 5 seconds. Movement holds one WASD key for 13–17 seconds, followed by a 1–5 second gap. Camera turns sample 60.1–199.9° and convert to relative pixels using window width and the configured scale; actual angles depend on game sensitivity. They last 2.5–3.5 seconds with a 2.5–3.5 second cooldown. Living HUD transitions, including loss of V, retain these loops. Pause, stop, and unknown or non-playing screens release held input; resume checks the current screen before sending more input.

After a confirmed `V_FIXED`, `LIVING_NO_V` requires the V icons to be absent and both the living controls and transform icon to match. It starts F, T, G once per round (40 ms taps, 0.3–0.7-second gaps), without interrupting WASD. Alt+1 repeats every 7–13 seconds for the rest of recognized play. The physical left-Alt/top-row-1 shortcut has an 80 ms modifier lead, 100 ms digit hold, and 60 ms modifier tail; pause between modifier and digit never sends an unmodified 1. Unknown screens suspend input without repeating the trigger; confirmed round exits reset it. Death, loading, and result recognition take priority.

Foreground loss during input enters the same 300-active-second recovery as capture failure. Held input is released and refocusing is retried every 250 ms; the user may manually bring the window back. Recovery reroutes the current screen. Expiry returns `window_recovery_timeout`, stopping the task and clearing the queue while the engine stays idle.

`MATCHING`, `READY_CASE`, `READY_START`, `DEATH_F`, `CONTINUE`, and `RETURN_LOBBY` are known waits and may remain on screen indefinitely. Unknown frames and continuous loading still time out after 300 active seconds. Active time excludes paused time: pause releases held input and freezes every task deadline; resume captures and reroutes the screen before more input is sent.

Template regions are defined under `assets/templates/hide_seek`; the detector normalizes frames to 1920 × 1080 and confirms recognized stages with a second observation. The supplied reference frames and unit tests cover detector routing and stage transitions. No live Identity V game session was used to validate these actions.

## Combat action schedules

Hunter match and rank share randomized movement, left/right clicks, and keyboard actions. F, R, and camera turns run every 10–15 seconds; right click every 25–35 seconds, left click every 5–30 seconds followed by E after 0.3–3 seconds, T every 40–50 seconds, Space every 5–30 seconds, and Q every 5–60 seconds. Three consecutive missing combat observations end the action worker; brief HUD misses preserve its timers and surrender clock.

Blackjack survivors add Space every 5–10 seconds to the existing Q/E/click/double-R/camera schedule. Blackjack hunters run Q and Space every 5–10 seconds alongside the existing right/T/left/E/camera combo. Scheduler deadlines use active time so sending actions does not accumulate countdown drift.

Menu cursor paths take 120–450 ms based on distance, with eased speed and exact endpoints. Combat turns use eased relative movement without zero-distance events. Login clicks no longer restore the old cursor position. Input still uses the normal Windows input APIs; these changes improve motion continuity and do not provide an undetectability guarantee.
