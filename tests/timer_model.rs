//! Finite-state refinement check against the Rust module used by the desktop app.

use std::collections::{HashSet, VecDeque};
use std::time::Duration;

mod production {
    include!("../src/timer.rs");

    pub(super) fn with_count(mut timer: Timer, count: u64) -> Timer {
        timer.completed_work = count;
        timer
    }

    pub(super) fn clock_values(timer: &Timer) -> (Duration, Duration) {
        let full = Duration::from_secs(u64::from(timer.total_seconds()));
        match timer.clock {
            Clock::Ready => (full, Duration::ZERO),
            Clock::Waiting { delay } => (full, delay),
            Clock::Running { remaining } | Clock::Paused { remaining } => {
                (remaining, Duration::ZERO)
            }
        }
    }
}

use production::{Finish, Phase, Status, Timer, TimerConfig, TimerEvent};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct Settings {
    work_minutes: u32,
    rounds: u32,
    auto_work: bool,
    auto_break: bool,
}

impl Settings {
    fn config(self) -> TimerConfig {
        TimerConfig::from_minutes(
            self.work_minutes,
            2,
            3,
            self.rounds,
            self.auto_work,
            self.auto_break,
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Motion {
    Stopped,
    Scheduled,
    Moving,
    Held,
}

// The specification counts elapsed half-seconds. Even positions in the cycle
// are focus rounds. Odd positions are breaks. The chosen break survives edits
// to the target round count.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct Spec {
    settings: Settings,
    position: u32,
    final_break: bool,
    motion: Motion,
    age: u32,
}

#[derive(Debug, Clone, Copy)]
enum Input {
    Toggle,
    Reset,
    Skip,
    Elapse(u32),
    Configure(Settings),
}

fn half_seconds(n: u32) -> Duration {
    Duration::from_millis(u64::from(n) * 500)
}

impl Spec {
    fn initial(settings: Settings) -> Self {
        Self {
            settings,
            position: 0,
            final_break: false,
            motion: Motion::Stopped,
            age: 0,
        }
    }

    fn phase(&self) -> Phase {
        if self.position.is_multiple_of(2) {
            Phase::Work
        } else if self.final_break {
            Phase::LongBreak
        } else {
            Phase::ShortBreak
        }
    }

    fn length(&self) -> u32 {
        120 * match self.phase() {
            Phase::Work => self.settings.work_minutes,
            Phase::ShortBreak => 2,
            Phase::LongBreak => 3,
        }
    }

    fn displayed(&self) -> u32 {
        let age = match self.motion {
            Motion::Moving | Motion::Held => self.age,
            _ => 0,
        };
        self.length() / 2 - age / 2
    }

    fn observe(&self, timer: &Timer) -> bool {
        let status = match self.motion {
            Motion::Stopped => Status::Ready,
            Motion::Scheduled => Status::Waiting,
            Motion::Moving => Status::Running,
            Motion::Held => Status::Paused,
        };
        let (remaining, delay) = match self.motion {
            Motion::Scheduled => (self.length(), 3 - self.age),
            _ => (self.length() - self.age, 0),
        };
        let next_wake = match self.motion {
            Motion::Stopped | Motion::Held => None,
            Motion::Scheduled => Some(half_seconds(3 - self.age)),
            Motion::Moving => Some(half_seconds(2 - self.age % 2)),
        };
        timer.phase() == self.phase()
            && timer.status() == status
            && (1..=self.settings.rounds).contains(&timer.round())
            && timer.round() == self.position / 2 + 1
            && timer.total_seconds() == self.length() / 2
            && timer.remaining_seconds() == self.displayed()
            && timer.next_wake() == next_wake
            && production::clock_values(timer) == (half_seconds(remaining), half_seconds(delay))
    }

    fn step(&mut self, input: Input) -> (Option<TimerEvent>, u64) {
        let phase = self.phase();
        let mut earned = 0;
        let event = match input {
            Input::Toggle => match self.motion {
                Motion::Held => {
                    self.motion = Motion::Moving;
                    Some(TimerEvent::Resumed(phase))
                }
                Motion::Moving => {
                    self.motion = Motion::Held;
                    Some(TimerEvent::Paused(phase))
                }
                _ => {
                    self.age = 0;
                    self.motion = Motion::Moving;
                    Some(TimerEvent::Started(phase))
                }
            },
            Input::Reset => {
                self.age = 0;
                self.motion = Motion::Stopped;
                Some(TimerEvent::Reset(phase))
            }
            Input::Configure(new) => {
                let length = self.length();
                self.settings = new;
                let last_position_of_same_kind = 2 * (new.rounds - 1) + self.position % 2;
                self.position = self.position.min(last_position_of_same_kind);
                if self.length() != length {
                    return self.step(Input::Reset);
                }
                None
            }
            Input::Elapse(t) if self.motion == Motion::Scheduled => {
                if self.age + t >= 3 {
                    return self.step(Input::Toggle);
                }
                self.age += t;
                None
            }
            Input::Elapse(_) if self.motion != Motion::Moving => None,
            Input::Elapse(t) if self.age + t < self.length() => {
                let before = self.displayed();
                self.age += t;
                (before != self.displayed()).then_some(TimerEvent::Tick(phase))
            }
            Input::Skip | Input::Elapse(_) => {
                earned = u64::from(phase == Phase::Work);
                self.position = if self.final_break {
                    0
                } else {
                    (self.position + 1) % (2 * self.settings.rounds)
                };
                self.final_break =
                    self.position % 2 == 1 && self.position >= 2 * self.settings.rounds - 1;
                let next = self.phase();
                self.age = 0;
                self.motion = if (next == Phase::Work && self.settings.auto_work)
                    || (next != Phase::Work && self.settings.auto_break)
                {
                    Motion::Scheduled
                } else {
                    Motion::Stopped
                };
                let cause = if matches!(input, Input::Skip) {
                    Finish::Skipped
                } else {
                    Finish::Elapsed
                };
                Some(TimerEvent::Completed { phase, next, cause })
            }
        };
        (event, earned)
    }
}

fn apply(timer: &mut Timer, input: Input) -> Option<TimerEvent> {
    match input {
        Input::Toggle => timer.toggle(),
        Input::Reset => timer.reset(),
        Input::Skip => timer.skip(),
        Input::Elapse(t) => timer.advance(half_seconds(t)),
        Input::Configure(settings) => timer.configure(settings.config()),
    }
}

fn counterexample(parents: &[Option<(usize, Input)>], mut id: usize, input: Input) -> Vec<Input> {
    let mut trace = vec![input];
    while let Some((parent, previous)) = parents[id] {
        trace.push(previous);
        id = parent;
    }
    trace.reverse();
    trace
}

#[test]
fn all_reachable_timer_states_refine_the_cycle_specification() {
    let initial = Settings {
        work_minutes: 1,
        rounds: 1,
        auto_work: false,
        auto_break: false,
    };
    let mut inputs = vec![
        Input::Toggle,
        Input::Reset,
        Input::Skip,
        Input::Elapse(0),
        Input::Elapse(1),
        Input::Elapse(2),
        Input::Elapse(3),
        Input::Elapse(120),
        Input::Elapse(721),
    ];
    for work_minutes in 1..=2 {
        for rounds in 1..=2 {
            for auto_work in [false, true] {
                for auto_break in [false, true] {
                    inputs.push(Input::Configure(Settings {
                        work_minutes,
                        rounds,
                        auto_work,
                        auto_break,
                    }));
                }
            }
        }
    }

    let initial_state = (Timer::new(initial.config()), Spec::initial(initial));
    assert!(initial_state.1.observe(&initial_state.0));
    assert_eq!(initial_state.0.completed_work(), 0);
    let mut seen = HashSet::from([initial_state.clone()]);
    let mut queue = VecDeque::from([(0, initial_state)]);
    let mut parents = vec![None];
    let mut edges = 0u64;
    while let Some((id, (actual, spec))) = queue.pop_front() {
        for &input in &inputs {
            let mut next = actual.clone();
            let mut expected = spec.clone();
            let event = apply(&mut next, input);
            let (expected_event, earned) = expected.step(input);
            let reached_focus_deadline = spec.phase() == Phase::Work
                && spec.motion == Motion::Moving
                && matches!(input, Input::Elapse(t) if spec.age + t >= spec.length());
            let celebrated = event.is_some_and(TimerEvent::celebrates_focus);
            if event != expected_event
                || celebrated != reached_focus_deadline
                || next.completed_work() != earned
                || !expected.observe(&next)
            {
                panic!(
                    "Inputs {:?}\nexpected {expected:?}, {expected_event:?}, credit {earned}, celebrate={reached_focus_deadline}\ngot {next:?}, {event:?}, celebrate={celebrated}",
                    counterexample(&parents, id, input)
                );
            }
            edges += 1;
            // Total completions are an output counter. Verify each edge's exact
            // credit above, then identify states that differ only by that output.
            let successor = (production::with_count(next, 0), expected);
            if seen.insert(successor.clone()) {
                queue.push_back((parents.len(), successor));
                parents.push(Some((id, input)));
            }
        }
    }
    println!(
        "{} reachable states, {} transitions, {} input alternatives",
        seen.len(),
        edges,
        inputs.len()
    );
}

#[test]
fn successive_scheduled_wakes_finish_each_phase_on_its_deadline() {
    let cfg = TimerConfig::from_minutes(1, 2, 3, 2, true, true);
    let mut timer = Timer::new(cfg);
    timer.toggle();
    for (phase, seconds, next) in [
        (Phase::Work, 60, Phase::ShortBreak),
        (Phase::ShortBreak, 120, Phase::Work),
        (Phase::Work, 60, Phase::LongBreak),
        (Phase::LongBreak, 180, Phase::Work),
    ] {
        for remaining in (1..=seconds).rev() {
            assert_eq!(timer.remaining_seconds(), remaining);
            let delay = timer.next_wake().unwrap();
            assert_eq!(delay, Duration::from_secs(1));
            let expected = if remaining == 1 {
                TimerEvent::Completed {
                    phase,
                    next,
                    cause: Finish::Elapsed,
                }
            } else {
                TimerEvent::Tick(phase)
            };
            assert_eq!(timer.advance(delay), Some(expected));
        }
        let delay = timer.next_wake().unwrap();
        assert_eq!(delay, Duration::from_millis(1500));
        assert_eq!(timer.advance(delay), Some(TimerEvent::Started(next)));
    }
    assert_eq!(
        (timer.phase(), timer.round(), timer.completed_work()),
        (Phase::Work, 1, 2)
    );
}

#[test]
fn every_supported_target_gets_its_full_cycle() {
    for rounds in 1..=12 {
        for minutes in 1..=90 {
            let mut timer = Timer::new(TimerConfig::from_minutes(
                minutes,
                91 - minutes,
                minutes,
                rounds,
                false,
                false,
            ));
            for round in 1..=rounds {
                assert_eq!((timer.phase().as_str(), timer.round()), ("work", round));
                assert_eq!(timer.total_seconds(), 60 * minutes);
                timer.toggle();
                assert_eq!(
                    timer.advance(Duration::from_secs(u64::from(60 * minutes) - 1)),
                    Some(TimerEvent::Tick(Phase::Work))
                );
                assert_eq!(timer.remaining_seconds(), 1);
                let (next, length) = if round == rounds {
                    (Phase::LongBreak, minutes)
                } else {
                    (Phase::ShortBreak, 91 - minutes)
                };
                assert_eq!(
                    timer.advance(Duration::from_secs(1)),
                    Some(TimerEvent::Completed {
                        phase: Phase::Work,
                        next,
                        cause: Finish::Elapsed,
                    })
                );
                assert_eq!(
                    (timer.phase(), timer.round(), timer.completed_work()),
                    (next, round, u64::from(round))
                );
                assert_eq!(
                    (timer.status(), timer.remaining_seconds()),
                    (Status::Ready, length * 60)
                );
                assert_eq!(
                    timer.skip(),
                    Some(TimerEvent::Completed {
                        phase: next,
                        next: Phase::Work,
                        cause: Finish::Skipped,
                    })
                );
            }
            assert_eq!((timer.phase(), timer.round()), (Phase::Work, 1));
        }
    }
}

#[test]
fn editing_rounds_preserves_the_clock_and_finishes_the_selected_break() {
    let config = |rounds| TimerConfig::from_minutes(1, 2, 3, rounds, false, false);
    let mut timer = Timer::new(config(5));
    for _ in 0..6 {
        timer.skip();
    }
    assert_eq!((timer.phase(), timer.round()), (Phase::Work, 4));
    timer.skip();
    timer.toggle();
    timer.advance(Duration::from_millis(12_345));
    assert_eq!(timer.configure(config(3)), None);
    assert_eq!((timer.phase(), timer.round()), (Phase::ShortBreak, 3));
    assert_eq!(timer.total_seconds(), 120);
    let remaining = Duration::from_millis(107_655);
    assert_eq!(
        production::clock_values(&timer),
        (remaining, Duration::ZERO)
    );
    assert_eq!(
        timer.advance(remaining),
        Some(TimerEvent::Completed {
            phase: Phase::ShortBreak,
            next: Phase::Work,
            cause: Finish::Elapsed,
        })
    );
    assert_eq!((timer.round(), timer.completed_work()), (1, 4));
    for _ in 0..5 {
        timer.skip();
    }
    assert_eq!((timer.phase(), timer.round()), (Phase::LongBreak, 3));
    timer.toggle();
    timer.advance(Duration::from_secs(20));
    assert_eq!(timer.configure(config(12)), None);
    assert_eq!(
        (timer.phase(), timer.remaining_seconds()),
        (Phase::LongBreak, 160)
    );
    timer.skip();
    assert_eq!((timer.phase(), timer.round()), (Phase::Work, 1));
}

#[test]
fn credits_saturate_without_changing_the_cycle() {
    let cfg = TimerConfig::from_minutes(25, 5, 15, 1, false, false);
    let mut timer = production::with_count(Timer::new(cfg), u64::MAX - 1);
    for _ in 0..2 {
        timer.skip();
        assert_eq!(timer.completed_work(), u64::MAX);
        assert_eq!(timer.phase(), Phase::LongBreak);
        timer.skip();
        assert_eq!(timer.completed_work(), u64::MAX);
        assert_eq!(timer.phase(), Phase::Work);
    }
}

#[test]
fn commands_replace_delayed_starts_and_preserve_fractional_seconds() {
    let cfg = TimerConfig::from_minutes(0, u32::MAX, 0, 0, true, true);
    let mut timer = Timer::new(cfg);
    assert_eq!(timer.total_seconds(), 60);
    timer.skip();
    assert_eq!(
        (timer.phase(), timer.status()),
        (Phase::LongBreak, Status::Waiting)
    );
    assert_eq!(timer.advance(Duration::from_nanos(1_499_999_999)), None);
    assert_eq!(
        timer.advance(Duration::from_nanos(1)),
        Some(TimerEvent::Started(Phase::LongBreak))
    );
    assert_eq!(timer.advance(Duration::from_nanos(999_999_999)), None);
    assert_eq!(timer.toggle(), Some(TimerEvent::Paused(Phase::LongBreak)));
    assert_eq!(timer.advance(Duration::MAX), None);
    assert_eq!(timer.toggle(), Some(TimerEvent::Resumed(Phase::LongBreak)));
    assert_eq!(
        timer.advance(Duration::from_nanos(1)),
        Some(TimerEvent::Tick(Phase::LongBreak))
    );
    assert_eq!(timer.remaining_seconds(), 59);
    assert_eq!(
        timer.advance(Duration::MAX),
        Some(TimerEvent::Completed {
            phase: Phase::LongBreak,
            next: Phase::Work,
            cause: Finish::Elapsed,
        })
    );
    timer.reset();
    assert_eq!(timer.advance(Duration::MAX), None);
    assert_eq!(
        (timer.status(), timer.remaining_seconds()),
        (Status::Ready, 60)
    );
    let largest = TimerConfig::from_minutes(0, u32::MAX, 0, u32::MAX, true, true);
    timer.configure(largest);
    timer.skip();
    assert_eq!(timer.remaining_seconds(), 5400);
    timer.configure(TimerConfig::from_minutes(0, 1, 0, 12, true, true));
    assert_eq!(
        (timer.status(), timer.remaining_seconds()),
        (Status::Ready, 60)
    );
    assert_eq!(timer.advance(Duration::MAX), None);
}
