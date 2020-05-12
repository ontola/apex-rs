use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use humantime::format_duration;
use std::collections::VecDeque;
use std::io::{stdout, StdoutLock, Write};
use std::ops::Div;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::*;
use tokio::task;

struct Reporter {
    msg_count: u32,
    error_count: u32,
    msg_per_second: f64,
    acc_poll_time: Duration,
    acc_parse_time: Duration,
    acc_delta_time: Duration,
    acc_fetch_time: Duration,
    acc_insert_time: Duration,
    last_messages: VecDeque<Instant>,
    last_timing: MessageTiming,
}

pub async fn report(rx: &mut Receiver<Result<MessageTiming, ErrorKind>>) -> Result<(), ()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    println!("Reported started");
    let mut reporter = Reporter::default();
    let mut io_limiter: i8 = 0;

    loop {
        let msg = rx.recv().await.unwrap();
        let reporter_start = Instant::now();
        reporter.update_processing_rate();

        match msg {
            Ok(timing) => reporter.add_timing(timing),
            Err(e) => reporter.add_error(e),
        }

        io_limiter = (io_limiter + 1) % 5;
        if io_limiter > 0 {
            task::yield_now().await;
            continue;
        }

        reporter.print_timing_report(&mut stdout);
        stdout
            .write_all(
                humanize("Reporter", Instant::now().duration_since(reporter_start)).as_bytes(),
            )
            .unwrap();

        stdout.flush().unwrap();
        task::yield_now().await;
    }
}

impl Reporter {
    fn update_processing_rate(&mut self) {
        let last_message = self.last_messages.pop_front().unwrap();
        let current_time = Instant::now();
        self.last_messages.push_back(current_time);
        self.msg_per_second = 1000f64 / current_time.duration_since(last_message).as_secs_f64();
        self.msg_count += 1;
    }

    fn add_timing(&mut self, timing: MessageTiming) {
        self.acc_poll_time += timing.poll_time;
        self.acc_parse_time += timing.parse_time;
        self.acc_delta_time += timing.delta_total();
        self.acc_fetch_time += timing.fetch_time;
        self.acc_insert_time += timing.insert_time;
        self.last_timing = timing;
    }

    fn add_error(&mut self, _: ErrorKind) {
        self.error_count += 1;
    }

    fn default() -> Reporter {
        let mut r = Reporter {
            msg_count: 0u32,
            error_count: 0,
            msg_per_second: 0.0,
            acc_poll_time: Duration::new(0, 0),
            acc_parse_time: Duration::new(0, 0),
            acc_delta_time: Duration::new(0, 0),
            acc_fetch_time: Duration::new(0, 0),
            acc_insert_time: Duration::new(0, 0),
            last_timing: MessageTiming::new(),
            last_messages: VecDeque::new(),
        };

        r.last_messages.reserve_exact(1_000);
        for _ in 0..1_000 {
            r.last_messages.push_back(Instant::now())
        }

        r
    }

    fn print_timing_report(&self, stdout: &mut StdoutLock) {
        write!(
            stdout,
            "\nProcessed: {} with {} errors at {:.2}/s\n",
            self.msg_count, self.error_count, self.msg_per_second
        )
        .unwrap();

        let last_timing = create_table(
            "Last timing",
            vec![
                ("poll", self.last_timing.poll_time),
                ("parse", self.last_timing.parse_time),
                ("fetch", self.last_timing.fetch_time),
                ("delta", self.last_timing.delta_total()),
                ("insert", self.last_timing.insert_time),
            ],
        );
        let last_delta = create_table(
            "Last delta",
            vec![
                ("setup", self.last_timing.delta_time.setup_time),
                ("sort", self.last_timing.delta_time.sort_time),
                ("remove", self.last_timing.delta_time.remove_time),
                ("replace", self.last_timing.delta_time.replace_time),
                ("add", self.last_timing.delta_time.add_time),
            ],
        );

        let avg_poll = self.acc_poll_time.div(self.msg_count);
        let avg_parse = self.acc_parse_time.div(self.msg_count);
        let avg_fetch = self.acc_fetch_time.div(self.msg_count);
        let avg_delta = self.acc_delta_time.div(self.msg_count);
        let avg_insert = self.acc_insert_time.div(self.msg_count);
        let avg = create_table(
            "Avg timing",
            vec![
                ("poll", self.acc_poll_time.div(self.msg_count)),
                ("parse", self.acc_parse_time.div(self.msg_count)),
                ("fetch", self.acc_fetch_time.div(self.msg_count)),
                ("delta", self.acc_delta_time.div(self.msg_count)),
                ("insert", self.acc_insert_time.div(self.msg_count)),
            ],
        );

        let total = create_table(
            "Total timing",
            vec![
                ("poll", self.acc_poll_time),
                ("parse", self.acc_parse_time),
                ("fetch", self.acc_fetch_time),
                ("delta", self.acc_delta_time),
                ("insert", self.acc_insert_time),
            ],
        );

        let grand_total = create_table(
            "Grand total",
            vec![
                (
                    "total",
                    self.acc_poll_time
                        + self.acc_parse_time
                        + self.acc_fetch_time
                        + self.acc_delta_time
                        + self.acc_insert_time,
                ),
                (
                    "avg",
                    avg_poll + avg_parse + avg_fetch + avg_delta + avg_insert,
                ),
            ],
        );

        stdout.write_all(last_timing.as_bytes()).unwrap();
        stdout.write_all(last_delta.as_bytes()).unwrap();
        stdout.write_all(avg.as_bytes()).unwrap();
        stdout.write_all(total.as_bytes()).unwrap();
        stdout.write_all(grand_total.as_bytes()).unwrap();
    }
}

fn create_table(row_name: &str, columns: Vec<(&str, Duration)>) -> String {
    let test = columns
        .into_iter()
        .map(|(l, t)| humanize(l, t))
        .collect::<String>();

    format!("{:<15} ‖ {}\n", row_name, test,)
}

#[inline]
fn humanize(label: &str, time: Duration) -> String {
    format!("{:.<10}.{:<27}| ", label, format_duration(time).to_string())
}
