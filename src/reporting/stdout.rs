use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::reporting::reporter::Reporter;
use humantime::format_duration;
use prometheus::core::Atomic;
use std::io::{stdout, StdoutLock, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc::*;
use tokio::task;

pub async fn report_stdout(
    rx: &mut Receiver<Result<MessageTiming, ErrorKind>>,
) -> Result<(), String> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    println!("Reported started");
    let reporter = Reporter::default();
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

fn create_table(row_name: &str, columns: Vec<(&str, Duration)>) -> String {
    let test = columns
        .into_iter()
        .map(|(l, t)| humanize(l, t))
        .collect::<String>();

    format!("{:<15} ‖ {}\n", row_name, test,)
}

#[inline]
pub(crate) fn humanize(label: &str, time: Duration) -> String {
    format!("{:.<10}.{:<27}| ", label, format_duration(time).to_string())
}

impl Reporter {
    fn print_timing_report(&self, stdout: &mut StdoutLock) {
        write!(
            stdout,
            "\nProcessed: {} with {} errors at {:.2}/s\n",
            self.msg_count.get(),
            self.error_count.get(),
            self.msg_per_second.get()
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

        let msg_count = self.msg_count.get();
        let acc_poll = self.acc_poll_time.get();
        let avg_poll = acc_poll / msg_count as f64;
        let acc_parse = self.acc_parse_time.get();
        let avg_parse = acc_parse / msg_count as f64;
        let acc_fetch = self.acc_fetch_time.get();
        let avg_fetch = acc_fetch / msg_count as f64;
        let acc_delta = self.acc_delta_time.get();
        let avg_delta = acc_delta / msg_count as f64;
        let acc_insert = self.acc_insert_time.get();
        let avg_insert = acc_insert / msg_count as f64;
        let avg = create_table(
            "Avg timing",
            vec![
                ("poll", Duration::from_millis(acc_poll as u64 / msg_count)),
                ("parse", Duration::from_millis(acc_parse as u64 / msg_count)),
                ("fetch", Duration::from_millis(acc_fetch as u64 / msg_count)),
                ("delta", Duration::from_millis(acc_delta as u64 / msg_count)),
                (
                    "insert",
                    Duration::from_millis(acc_insert as u64 / msg_count),
                ),
            ],
        );

        let total = create_table(
            "Total timing",
            vec![
                ("poll", Duration::from_millis(acc_poll as u64)),
                ("parse", Duration::from_millis(acc_parse as u64)),
                ("fetch", Duration::from_millis(acc_fetch as u64)),
                ("delta", Duration::from_millis(acc_delta as u64)),
                ("insert", Duration::from_millis(acc_insert as u64)),
            ],
        );

        let acc_sum = acc_poll + acc_parse + acc_fetch + acc_delta + acc_insert;
        let avg_sum = avg_poll + avg_parse + avg_fetch + avg_delta + avg_insert;
        let grand_total = create_table(
            "Grand total",
            vec![
                ("total", Duration::from_millis(acc_sum as u64)),
                ("avg", Duration::from_millis(avg_sum as u64)),
            ],
        );

        stdout.write_all(last_timing.as_bytes()).unwrap();
        stdout.write_all(last_delta.as_bytes()).unwrap();
        stdout.write_all(avg.as_bytes()).unwrap();
        stdout.write_all(total.as_bytes()).unwrap();
        stdout.write_all(grand_total.as_bytes()).unwrap();
    }
}
