use crate::reporting::prometheus_u64::U64Counter;
use prometheus::{register_histogram, Histogram};

pub struct Metrics {
    pub message_count_metric: U64Counter,
    pub error_count_metric: U64Counter,
    pub poll_time_metric: Histogram,
    pub parse_time_metric: Histogram,
    pub delta_time_metric: Histogram,
    pub fetch_time_metric: Histogram,
    pub insert_time_metric: Histogram,
}

impl Default for Metrics {
    fn default() -> Metrics {
        let message_count_metric = register_u64_counter!(
            "invalidator_messages_message",
            "The amount of messages processed"
        )
        .expect("can not create metric message");
        let error_count_metric = register_u64_counter!(
            "invalidator_messages_error",
            "The amount of error messages processed"
        )
        .expect("can not create metric error");

        let poll_time_metric = register_histogram!(
            "invalidator_timings_poll",
            "The accumulative time spent polling for messages"
        )
        .expect("can not create metric invalidator_timings_poll");
        let parse_time_metric = register_histogram!(
            "invalidator_timings_parse",
            "The accumulative time spent parsing messages"
        )
        .expect("can not create metric invalidator_timings_parse");
        let delta_time_metric = register_histogram!(
            "invalidator_timings_delta",
            "The accumulative time spent diffing messages"
        )
        .expect("can not create metric invalidator_timings_delta");
        let fetch_time_metric = register_histogram!(
            "invalidator_timings_fetch",
            "The accumulative time spent fetching data from db"
        )
        .expect("can not create metric invalidator_timings_fetch");
        let insert_time_metric = register_histogram!(
            "invalidator_timings_insert",
            "The accumulative time spent inserting data to db"
        )
        .expect("can not create metric invalidator_timings_insert");

        Metrics {
            message_count_metric,
            error_count_metric,
            poll_time_metric,
            parse_time_metric,
            delta_time_metric,
            fetch_time_metric,
            insert_time_metric,
        }
    }
}
