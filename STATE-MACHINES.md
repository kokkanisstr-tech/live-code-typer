# Runtime state map

This file records the app's runtime state machines so UI, input, room, and persistence changes can be reviewed against the same transition map. Runtime state is not part of the user's saved library unless explicitly marked.

## Typing mode

`Library::typing_state` is the authoritative state in both Original and Demo. It is skipped during serialization, so existing `library.json` files remain compatible.

```text
Idle --start manually--> SignalDriven --last character--> Idle
Idle --start automatic, zero delay--> Automatic --last character--> Idle
Idle --start automatic, delay--> WaitingUntil --deadline--> Automatic
any active mode --pause/reset/edit/select/replace--> Idle
```

Only `SignalDriven` consumes a random keyboard signal. `WaitingUntil` and `Automatic` consume those signals without allowing them through, and the automatic worker emits characters only after the deadline. A revision/cursor mismatch rejects stale work. These invariants are encoded in `TypingState` and `Library::commit_input`.

## Other discovered state machines

- **Room client lifecycle:** disconnected -> connecting -> connected -> reconnecting -> connected; errors/disconnect return to a non-connected state. Today this is split across a client `Option`, a connected `bool`, a free-form status string, and an error `Option` in `ui.rs`. Keep status text as presentation; a future change can replace the flags with a typed room state.
- **Room assignment:** no assignment -> assigned task -> delayed automatic typing -> completed/paused; a new assignment cancels the old run and resets the selected remote snippet. This is coordinated by `RoomClientEvent`, `Library::apply_remote_assignment`, and the typing mode.
- **Hotkey capture:** idle -> waiting for a valid modified key -> accepted/cancelled. `capturing_hotkey: Option<HotkeyAction>` already represents this compactly; Escape cancels and invalid modifier-only presses keep capture active while showing an error.
- **Settings/help/room windows:** each is open or closed. These are independent overlays; they should stay independent until the app adds an exclusive navigation shell.
- **Persistence debounce:** clean -> dirty with a deadline -> saved; save failure remains dirty and retries later. `dirty` plus `save_at` is deliberate because the retry deadline changes after failures.
- **Keyboard hook:** tracks held modifiers and suppressed key releases so injected output does not recursively trigger itself and consumed key presses do not leak into the target app. Treat each key press/release pair as a transition pair; do not simplify the sets into a single boolean.
- **Room server participant/task revisions:** join/leave, assignment, command, and polling update revision counters; clients apply only new assignment/command revisions. These counters are protocol state and must remain monotonic across persisted room data.

## Change rule

When editing one of these flows, update its transition map, user-visible status, and E2E scenario together. Preserve the serialized library and room protocol unless an explicit migration/version change is added.
