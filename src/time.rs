use std::fmt::{self, Display, Formatter};
use std::time::{Duration, Instant};

pub trait Timer {
    type Time: Display;

    fn time(&self) -> Self::Time;
}

pub struct InstantOutput(Duration);

impl Display for InstantOutput {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let minutes = self.0.as_secs() / 60;
        let hour = minutes / 60;
        let minute = minutes % 60;
        let second = self.0.as_secs() % 60;
        let milli = self.0.subsec_millis();
        let micro = self.0.subsec_micros() % 1_000;
        let nano = self.0.subsec_nanos() % 1_000;

        write!(
            f,
            "{hour:02}:{minute:02}:{second:02}:{milli:03}.{micro:03}.{nano:03}",
        )
    }
}

impl Timer for Instant {
    type Time = InstantOutput;

    fn time(&self) -> Self::Time {
        InstantOutput(self.elapsed())
    }
}
