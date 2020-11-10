use actix_web::{get, HttpResponse, Responder};
use prometheus::{linear_buckets, register_histogram, Encoder, Histogram, IntCounter, TextEncoder};

#[derive(Clone)]
pub(crate) struct Metrics {
    pub bulk: BulkMetrics,
}

#[derive(Clone)]
pub(crate) struct BulkMetrics {
    pub request_count: IntCounter,
    pub resources_count: Histogram,

    pub parse_time: Histogram,
    pub lookup_time: Histogram,
    pub sort_time: Histogram,
    pub authorize_tot_time: Histogram,
    pub authorize_timing: AuthorizeMetrics,
    pub serialize_time: Histogram,
}

#[derive(Clone)]
pub(crate) struct AuthorizeMetrics {
    pub authorize_fetch_time: Histogram,
    pub authorize_parse_time: Histogram,
    pub authorize_process_time: Histogram,
    pub authorize_finish_time: Histogram,
}

impl Default for Metrics {
    fn default() -> Self {
        Metrics {
            bulk: BulkMetrics::default(),
        }
    }
}

#[get("/link-lib/metrics")]
pub(crate) async fn metrics() -> impl Responder {
    let mut buffer = vec![];
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    HttpResponse::Ok().body(buffer)
}

impl Default for BulkMetrics {
    fn default() -> Self {
        let request_count = register_int_counter!(
            "http_bulk_request_count",
            "The total number of requests requested"
        )
        .expect("can not create metric http_bulk_request_count");

        let resources_opts = histogram_opts!(
            "http_bulk_resources",
            "The number of resources requested per request",
            linear_buckets(0.0, 1.0, 100).unwrap()
        );
        let resources_count =
            register_histogram!(resources_opts).expect("can not create metric http_bulk_resources");

        let parse_time =
            register_histogram!("http_bulk_parse_time", "The time spent parsing the request")
                .expect("can not create metric invalidator_timings_poll");
        let lookup_time = register_histogram!(
            "http_bulk_lookup_time",
            "The time spent checking which resources are in the db"
        )
        .expect("can not create metric invalidator_timings_parse");
        let sort_time = register_histogram!(
            "http_bulk_sort_time",
            "The time spent figuring out which resources need to be authorized and/or fetched"
        )
        .expect("can not create metric invalidator_timings_delta");
        let authorize_tot_time = register_histogram!(
            "http_bulk_authorize_tot_time",
            "The total time spent authorizing & fetching data at the resource server"
        )
        .expect("can not create metric invalidator_timings_fetch");
        let serialize_time = register_histogram!(
            "http_bulk_serialize_time",
            "The time spent serializing the response"
        )
        .expect("can not create metric invalidator_timings_insert");
        let authorize_timing = AuthorizeMetrics::default();

        BulkMetrics {
            request_count,
            resources_count,

            parse_time,
            lookup_time,
            sort_time,
            authorize_tot_time,
            authorize_timing,
            serialize_time,
        }
    }
}

impl Default for AuthorizeMetrics {
    fn default() -> Self {
        let authorize_fetch_time = register_histogram!(
            "http_bulk_authorize_fetch_time",
            "Time spent calling the endpoints for authorization and/or fetching resources"
        )
        .expect("can not create metric http_bulk_authorize_fetch_time");
        let authorize_parse_time = register_histogram!(
            "http_bulk_authorize_parse_time",
            "Time spent parsing the fetched resources"
        )
        .expect("can not create metric http_bulk_authorize_parse_time");
        let authorize_process_time = register_histogram!(
            "http_bulk_authorize_process_time",
            "Time spent processing the data in the fetched resources (applying and storing)"
        )
        .expect("can not create metric http_bulk_authorize_process_time");
        let authorize_finish_time = register_histogram!(
            "http_bulk_authorize_finish_time",
            "Time spent consolidating the results back into the collected resources"
        )
        .expect("can not create metric http_bulk_authorize_finish_time");

        AuthorizeMetrics {
            authorize_fetch_time,
            authorize_finish_time,
            authorize_parse_time,
            authorize_process_time,
        }
    }
}
