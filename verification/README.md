# Timer verification

Run the checker from the repository root with the Rust toolchain installed.

```sh
cargo test --locked --offline --manifest-path verification/Cargo.toml -- --nocapture
```

The standalone manifest uses only the Rust standard library. The desktop package runs the same checks through `cargo test --test timer_model`.

## What the checker establishes

[The checker](../tests/timer_model.rs) compiles the [desktop timer](../src/timer.rs) directly. It compares every reachable transition in the finite model with an independent specification. The search continues until every successor has already been visited. A mismatch prints a shortest input sequence from launch.

The specification represents a cycle as numbered positions. Even positions are focus rounds. Odd positions are breaks. A flag records whether a break ends the cycle. The specification counts elapsed half-seconds from the start of a phase or delay. The application stores the current phase, round, and remaining time.

For each input, the checker compares the phase, round number, completion credit, status, displayed seconds, exact remaining duration, pending start delay, next wake, and emitted event. The checks cover these behaviors.

- An uninterrupted round emits one completion at its deadline. A late callback completes at most one round.
- A manual skip takes the same cycle transition as expiry. Either credits one focus round when leaving work. Leaving a break adds no credit.
- The current round stays between one and the configured target. Lowering the target caps the current round immediately. A short break normally advances one round. If a target edit makes that break the last round, it leads directly to round one. A long break always leads to round one, including after a target increase. Editing the target preserves the current break type and clock.
- Pausing freezes the remaining duration. Resuming preserves it. Reset restores the full duration of the same phase and cancels a scheduled start.
- Each transition samples the auto-start preference for its destination. A pending start waits 1.5 seconds. Starting manually consumes that pending start.
- An edit to the current phase duration resets that phase. Other settings edits preserve its clock.

## Bounds and assumptions

The search starts at the first work round. Every state accepts each of 25 inputs.

| Inputs | Values |
| --- | --- |
| User controls | Toggle, reset, skip |
| Monotonic time advances | 0, 0.5, 1, 1.5, 60, or 360.5 seconds |
| Settings updates | All 16 combinations of a 1- or 2-minute focus duration, a target of 1 or 2 rounds, and both auto-start flags |

The short break lasts two minutes and the long break lasts three. The half-second input can reach every half-second clock position. The longer inputs exercise delayed callbacks and both sides of deadlines. The search visits **33,856 states and 846,400 transitions**. It covers arbitrarily long input sequences within this alphabet.

The search identifies states that differ only in the lifetime focus count. The implementation reads that count only to increment it with saturating addition. The checker compares the credit on every transition before discarding the previous total. A separate test starts at `u64::MAX - 1` and completes two cycles to check saturation.

Additional tests run a whole cycle for each of the 12 round targets at each of the 90 supported minute values. A regression test lowers the target from five to three during the fourth short break, checks the remaining time to the millisecond, then raises the target during the next long break. Other tests exercise nanosecond boundaries, an oversized clock jump, invalid settings, and a full automatic cycle driven solely by the deadlines the timer requests.

The formal result applies to the stated finite model. Other clock values and settings combinations have the supplemental test coverage described above. Native window events, operating-system scheduling, notifications, sound, sockets, and preferences storage are outside this check.

## Counterexample

A negative control changed the long-break decision in a temporary copy of the production timer from `round >= configured_rounds` to `round > configured_rounds`. The checker rejected the mutated timer after one input, `Skip`. With a target of one, the specification selected a long break and the mutated code selected a short break.

A second negative control allowed a short break to advance the round when the round already equaled the target. The checker rejected it after four inputs. Set the target to two, skip into a short break, lower the target to one, and skip the break. The mutated timer started work at round two of one.
