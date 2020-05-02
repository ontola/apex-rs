use crate::importing::events::MessageTiming;
use humantime::format_duration;
use std::collections::VecDeque;
use std::io::{stdout, Write};
use std::ops::Div;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::*;
use tokio::task;

pub async fn report(rx: &mut Receiver<MessageTiming>) -> Result<(), ()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    println!("Reported started");
    let mut msg_count = 0u32;
    let mut acc_poll_time = Duration::new(0, 0);
    let mut acc_parse_time = Duration::new(0, 0);
    let mut acc_delta_time = Duration::new(0, 0);
    let mut acc_fetch_time = Duration::new(0, 0);
    let mut acc_insert_time = Duration::new(0, 0);
    let mut last_messages: VecDeque<Instant> = VecDeque::new();
    last_messages.reserve_exact(1_000);
    for _ in 0..1_000 {
        last_messages.push_back(Instant::now())
    }
    let mut io_limiter: i8 = 0;

    loop {
        let msg = rx.recv().await.unwrap();
        let reporter_time = Instant::now();
        let last_message = last_messages.pop_front().unwrap();
        let current_time = Instant::now();
        last_messages.push_back(current_time);
        let msg_per_second = 1000f64 / current_time.duration_since(last_message).as_secs_f64();
        msg_count += 1;

        acc_poll_time += msg.poll_time;
        acc_parse_time += msg.parse_time;
        acc_delta_time += msg.delta_total();
        acc_fetch_time += msg.fetch_time;
        acc_insert_time += msg.insert_time;

        io_limiter = (io_limiter + 1) % 5;

        if io_limiter > 0 {
            continue;
        }

        write!(
            stdout,
            "\nProcessed: {} with {:.2}/s\n",
            msg_count, msg_per_second
        )
        .unwrap();

        let last_timing = create_table(
            "Last timing",
            vec![
                ("poll", msg.poll_time),
                ("parse", msg.parse_time),
                ("fetch", msg.fetch_time),
                ("delta", msg.delta_total()),
                ("insert", msg.insert_time),
            ],
        );
        let last_delta = create_table(
            "Last delta",
            vec![
                ("setup", msg.delta_time.setup_time),
                ("sort", msg.delta_time.sort_time),
                ("remove", msg.delta_time.remove_time),
                ("replace", msg.delta_time.replace_time),
                ("add", msg.delta_time.add_time),
            ],
        );

        let avg_poll = acc_poll_time.div(msg_count);
        let avg_parse = acc_parse_time.div(msg_count);
        let avg_fetch = acc_fetch_time.div(msg_count);
        let avg_delta = acc_delta_time.div(msg_count);
        let avg_insert = acc_insert_time.div(msg_count);
        let avg = create_table(
            "Avg timing",
            vec![
                ("poll", acc_poll_time.div(msg_count)),
                ("parse", acc_parse_time.div(msg_count)),
                ("fetch", acc_fetch_time.div(msg_count)),
                ("delta", acc_delta_time.div(msg_count)),
                ("insert", acc_insert_time.div(msg_count)),
            ],
        );

        let total = create_table(
            "Total timing",
            vec![
                ("poll", acc_poll_time),
                ("parse", acc_parse_time),
                ("fetch", acc_fetch_time),
                ("delta", acc_delta_time),
                ("insert", acc_insert_time),
            ],
        );

        let grand_total = create_table(
            "Grand total",
            vec![
                (
                    "total",
                    acc_poll_time
                        + acc_parse_time
                        + acc_fetch_time
                        + acc_delta_time
                        + acc_insert_time,
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
        stdout
            .write_all(
                humanize("Reporter", Instant::now().duration_since(reporter_time)).as_bytes(),
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
fn humanize(label: &str, time: Duration) -> String {
    format!("{:.<10}.{:<27}| ", label, format_duration(time).to_string())
}
