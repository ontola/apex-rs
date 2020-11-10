use crate::serving::metrics::Metrics;
use crate::serving::timings::BulkTiming;

#[derive(Clone)]
pub(crate) struct Reporter {
    pub(crate) metrics: Metrics,
}

impl Reporter {
    pub(crate) fn register_bulk_request(&self) {
        self.metrics.bulk.request_count.inc();
    }

    pub(crate) fn register_bulk_resource_count(&self, count: usize) {
        debug!(target: "apex", "Adding {} as {} to resources hist", count, count as f64);
        let s_count = self.metrics.bulk.resources_count.get_sample_count();
        let s_sum = self.metrics.bulk.resources_count.get_sample_sum();
        debug!(target: "apex", "[before] count: {}, sum: {}", s_count, s_sum);
        self.metrics.bulk.resources_count.observe(count as f64);
        let s_count = self.metrics.bulk.resources_count.get_sample_count();
        let s_sum = self.metrics.bulk.resources_count.get_sample_sum();
        debug!(target: "apex", "[after] count: {}, sum: {}", s_count, s_sum);
    }

    pub(crate) fn add_bulk_timing(&self, timing: BulkTiming) {
        self.metrics
            .bulk
            .parse_time
            .observe(timing.parse_time.as_secs_f64());
        self.metrics
            .bulk
            .lookup_time
            .observe(timing.lookup_time.as_secs_f64());
        self.metrics
            .bulk
            .sort_time
            .observe(timing.sort_time.as_secs_f64());
        match timing.authorize_timing {
            Some(auth_timing) => {
                let fetch = auth_timing.authorize_fetch_time.as_secs_f64();
                let parse = auth_timing.authorize_parse_time.as_secs_f64();
                let process = auth_timing.authorize_process_time.as_secs_f64();
                let finish = auth_timing.authorize_finish_time.as_secs_f64();

                self.metrics
                    .bulk
                    .authorize_timing
                    .authorize_fetch_time
                    .observe(fetch);
                self.metrics
                    .bulk
                    .authorize_timing
                    .authorize_parse_time
                    .observe(parse);
                self.metrics
                    .bulk
                    .authorize_timing
                    .authorize_process_time
                    .observe(process);
                self.metrics
                    .bulk
                    .authorize_timing
                    .authorize_finish_time
                    .observe(finish);
                self.metrics
                    .bulk
                    .authorize_tot_time
                    .observe(fetch + parse + process + finish)
            }
            None => self.metrics.bulk.authorize_tot_time.observe(0.0),
        }
        self.metrics
            .bulk
            .serialize_time
            .observe(timing.serialize_time.as_secs_f64());
    }
}

impl Default for Reporter {
    fn default() -> Reporter {
        Reporter {
            metrics: Metrics::default(),
        }
    }
}
