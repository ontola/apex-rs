use crate::reporting::stdout::humanize;
use std::time::Duration;

pub(crate) struct BulkTiming {
    /// Parsing the request
    pub parse_time: Duration,
    /// Checking which resources are in the db
    pub lookup_time: Duration,
    /// Figuring out which resources need to be authorized and/or fetched
    pub sort_time: Duration,
    /// Breakdown of steps withing authorizing
    pub authorize_timing: Option<AuthorizeTiming>,
    /// Serializing the response
    pub serialize_time: Duration,
}

pub(crate) struct AuthorizeTiming {
    /// Calling the endpoints for authorization and/or fetching resources
    pub authorize_fetch_time: Duration,
    /// Parsing the fetched resources
    pub authorize_parse_time: Duration,
    /// Processing the data in the fetched resources (applying and storing)
    pub authorize_process_time: Duration,
    /// Consolidating the results back into the collected resources
    pub authorize_finish_time: Duration,
}

impl BulkTiming {
    pub fn from_durations(
        parse_time: Duration,
        lookup_time: Duration,
        sort_time: Duration,
        authorize_timing: Option<AuthorizeTiming>,
        serialize_time: Duration,
    ) -> BulkTiming {
        BulkTiming {
            parse_time,
            lookup_time,
            sort_time,
            authorize_timing,
            serialize_time,
        }
    }

    pub fn report(&self) {
        if cfg!(debug_assertions) {
            log_timing(self)
        }
    }
}

impl AuthorizeTiming {
    pub fn from_durations(
        fetch: Duration,
        parse: Duration,
        process: Duration,
        finish: Duration,
    ) -> AuthorizeTiming {
        AuthorizeTiming {
            authorize_fetch_time: fetch,
            authorize_parse_time: parse,
            authorize_process_time: process,
            authorize_finish_time: finish,
        }
    }
}

impl Default for AuthorizeTiming {
    fn default() -> Self {
        AuthorizeTiming {
            authorize_fetch_time: Duration::new(0, 0),
            authorize_parse_time: Duration::new(0, 0),
            authorize_process_time: Duration::new(0, 0),
            authorize_finish_time: Duration::new(0, 0),
        }
    }
}

fn log_timing(timing: &BulkTiming) {
    let parse_msg = humanize("parse", timing.parse_time);
    let lookup_msg = humanize("lookup", timing.lookup_time);
    let sort_msg = humanize("sort", timing.sort_time);
    let auth_times = match &timing.authorize_timing {
        Some(a) => (
            a.authorize_fetch_time,
            a.authorize_parse_time,
            a.authorize_process_time,
            a.authorize_finish_time,
        ),
        None => (
            Duration::new(0, 0),
            Duration::new(0, 0),
            Duration::new(0, 0),
            Duration::new(0, 0),
        ),
    };
    let auth_msg = format!(
        "{}{}{}{}",
        humanize("auth fetch", auth_times.0),
        humanize("auth parse", auth_times.1),
        humanize("auth process", auth_times.2),
        humanize("auth finish", auth_times.3)
    );
    let serialize_msg = humanize("serialize", timing.serialize_time);

    debug!(target: "apex", "Bulk time: {}{}{}{}{}", parse_msg, lookup_msg, sort_msg, auth_msg, serialize_msg);
    let internal_time = timing.parse_time
        + timing.lookup_time
        + timing.sort_time
        + auth_times.1
        + auth_times.2
        + auth_times.3
        + timing.serialize_time;
    info!(target: "apex", "Bulk res: {}{}",
          humanize("internal", internal_time),
          humanize("external", auth_times.0),
    );
}
