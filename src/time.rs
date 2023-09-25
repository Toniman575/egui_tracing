use std::time::Duration;

pub trait DurationExt {
    fn display_ext(&self) -> String;
}

impl DurationExt for Duration {
    fn display_ext(&self) -> String {
        let minutes = self.as_secs() / 60;
        let hour = minutes / 60;
        let minute = minutes % 60;
        let second = self.as_secs() % 60;
        let milli = self.subsec_millis();
        let micro = self.subsec_micros() % 1_000;
        let nano = self.subsec_nanos() % 1_000;

        format!("{hour:02}:{minute:02}:{second:02}:{milli:03}.{micro:03}.{nano:03}",)
    }
}
