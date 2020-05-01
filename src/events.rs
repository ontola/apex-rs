use std::ops::AddAssign;
use std::time::Duration;

pub struct MessageTiming {
    pub poll_time: Duration,
    pub parse_time: Duration,
    pub fetch_time: Duration,
    pub delta_time: DeltaProcessingTiming,
    pub insert_time: Duration,
}

pub struct DeltaProcessingTiming {
    pub setup_time: Duration,
    pub sort_time: Duration,
    pub remove_time: Duration,
    pub replace_time: Duration,
    pub add_time: Duration,
}

impl MessageTiming {
    pub fn new() -> MessageTiming {
        MessageTiming {
            poll_time: Duration::new(0, 0),
            parse_time: Duration::new(0, 0),
            fetch_time: Duration::new(0, 0),
            delta_time: DeltaProcessingTiming::new(),
            insert_time: Duration::new(0, 0),
        }
    }

    pub fn delta_total(&self) -> Duration {
        self.delta_time.setup_time
            + self.delta_time.sort_time
            + self.delta_time.remove_time
            + self.delta_time.replace_time
            + self.delta_time.add_time
    }
}

impl DeltaProcessingTiming {
    pub fn new() -> DeltaProcessingTiming {
        DeltaProcessingTiming {
            setup_time: Duration::new(0, 0),
            sort_time: Duration::new(0, 0),
            remove_time: Duration::new(0, 0),
            replace_time: Duration::new(0, 0),
            add_time: Duration::new(0, 0),
        }
    }
}

impl AddAssign for DeltaProcessingTiming {
    fn add_assign(&mut self, other: Self) {
        *self = Self {
            setup_time: self.setup_time + other.setup_time,
            sort_time: self.sort_time + other.sort_time,
            remove_time: self.remove_time + other.remove_time,
            replace_time: self.replace_time + other.replace_time,
            add_time: self.add_time + other.add_time,
        };
    }
}
