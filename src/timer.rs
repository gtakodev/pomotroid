use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Phase {
    Work,
    ShortBreak,
    LongBreak,
}

impl Phase {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Work => "work",
            Self::ShortBreak => "short-break",
            Self::LongBreak => "long-break",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    Ready,
    Waiting,
    Running,
    Paused,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TimerConfig {
    work: Duration,
    short_break: Duration,
    long_break: Duration,
    rounds: u32,
    auto_work: bool,
    auto_break: bool,
}

impl TimerConfig {
    pub fn from_minutes(
        work: u32,
        short_break: u32,
        long_break: u32,
        rounds: u32,
        auto_work: bool,
        auto_break: bool,
    ) -> Self {
        let duration = |minutes: u32| Duration::from_secs(u64::from(minutes.clamp(1, 90)) * 60);
        Self {
            work: duration(work),
            short_break: duration(short_break),
            long_break: duration(long_break),
            rounds: rounds.clamp(1, 12),
            auto_work,
            auto_break,
        }
    }

    fn duration(&self, phase: Phase) -> Duration {
        match phase {
            Phase::Work => self.work,
            Phase::ShortBreak => self.short_break,
            Phase::LongBreak => self.long_break,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimerEvent {
    Started(Phase),
    Resumed(Phase),
    Paused(Phase),
    Reset(Phase),
    Tick(Phase),
    Completed { phase: Phase, next: Phase },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Clock {
    Ready,
    Waiting { delay: Duration },
    Running { remaining: Duration },
    Paused { remaining: Duration },
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Timer {
    config: TimerConfig,
    phase: Phase,
    clock: Clock,
    round: u32,
    completed_work: u64,
}

impl Timer {
    pub fn new(config: TimerConfig) -> Self {
        Self {
            config,
            phase: Phase::Work,
            clock: Clock::Ready,
            round: 1,
            completed_work: 0,
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn status(&self) -> Status {
        match self.clock {
            Clock::Ready => Status::Ready,
            Clock::Waiting { .. } => Status::Waiting,
            Clock::Running { .. } => Status::Running,
            Clock::Paused { .. } => Status::Paused,
        }
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn completed_work(&self) -> u64 {
        self.completed_work
    }

    pub fn total_seconds(&self) -> u32 {
        self.config.duration(self.phase).as_secs() as u32
    }

    pub fn remaining_seconds(&self) -> u32 {
        match self.clock {
            Clock::Ready | Clock::Waiting { .. } => self.total_seconds(),
            Clock::Running { remaining } | Clock::Paused { remaining } => {
                whole_seconds_left(remaining)
            }
        }
    }

    pub fn next_wake(&self) -> Option<Duration> {
        match self.clock {
            Clock::Ready | Clock::Paused { .. } => None,
            Clock::Waiting { delay } => Some(delay),
            Clock::Running { remaining } if remaining.subsec_nanos() > 0 => {
                Some(Duration::from_nanos(u64::from(remaining.subsec_nanos())))
            }
            Clock::Running { .. } => Some(Duration::from_secs(1)),
        }
    }

    pub fn toggle(&mut self) -> Option<TimerEvent> {
        let (clock, event) = match self.clock {
            Clock::Ready | Clock::Waiting { .. } => (
                Clock::Running {
                    remaining: self.config.duration(self.phase),
                },
                TimerEvent::Started(self.phase),
            ),
            Clock::Running { remaining } => {
                (Clock::Paused { remaining }, TimerEvent::Paused(self.phase))
            }
            Clock::Paused { remaining } => (
                Clock::Running { remaining },
                TimerEvent::Resumed(self.phase),
            ),
        };
        self.clock = clock;
        Some(event)
    }

    pub fn reset(&mut self) -> Option<TimerEvent> {
        self.clock = Clock::Ready;
        Some(TimerEvent::Reset(self.phase))
    }

    pub fn skip(&mut self) -> Option<TimerEvent> {
        Some(self.complete())
    }

    pub fn configure(&mut self, config: TimerConfig) -> Option<TimerEvent> {
        let previous = self.config.duration(self.phase);
        self.config = config;
        self.round = self.round.min(config.rounds);
        if self.config.duration(self.phase) != previous {
            return self.reset();
        }
        None
    }

    /// A delayed callback finishes at most one phase. The next phase gets its full
    /// duration when the user sees it, even after the computer wakes from sleep.
    pub fn advance(&mut self, elapsed: Duration) -> Option<TimerEvent> {
        match self.clock {
            Clock::Ready | Clock::Paused { .. } => None,
            Clock::Waiting { delay } if elapsed >= delay => self.toggle(),
            Clock::Waiting { delay } => {
                self.clock = Clock::Waiting {
                    delay: delay - elapsed,
                };
                None
            }
            Clock::Running { remaining } if elapsed >= remaining => Some(self.complete()),
            Clock::Running { remaining } => {
                let next = remaining - elapsed;
                self.clock = Clock::Running { remaining: next };
                (whole_seconds_left(next) != whole_seconds_left(remaining))
                    .then_some(TimerEvent::Tick(self.phase))
            }
        }
    }

    fn complete(&mut self) -> TimerEvent {
        let phase = self.phase;
        self.phase = match phase {
            Phase::Work => {
                self.completed_work = self.completed_work.saturating_add(1);
                if self.round >= self.config.rounds {
                    Phase::LongBreak
                } else {
                    Phase::ShortBreak
                }
            }
            Phase::ShortBreak if self.round < self.config.rounds => {
                self.round += 1;
                Phase::Work
            }
            Phase::ShortBreak | Phase::LongBreak => {
                self.round = 1;
                Phase::Work
            }
        };
        let auto_start = match self.phase {
            Phase::Work => self.config.auto_work,
            _ => self.config.auto_break,
        };
        self.clock = if auto_start {
            Clock::Waiting {
                delay: Duration::from_millis(1500),
            }
        } else {
            Clock::Ready
        };
        TimerEvent::Completed {
            phase,
            next: self.phase,
        }
    }
}

fn whole_seconds_left(duration: Duration) -> u32 {
    duration.as_secs() as u32 + u32::from(duration.subsec_nanos() > 0)
}
