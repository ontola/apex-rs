use prometheus::{core::AtomicU64, core::GenericCounter};

pub(crate) type U64Counter = GenericCounter<AtomicU64>;

/// Create an [`U64Counter`] and registers to default registry.
///
/// View docs of `register_counter` for examples.
#[macro_export]
macro_rules! register_u64_counter {
    ($OPTS:expr) => {{
        let counter = U64Counter::with_opts($OPTS).unwrap();
        prometheus::register(Box::new(counter.clone())).map(|_| counter)
    }};

    ($NAME:expr, $HELP:expr) => {{
        register_u64_counter!(prometheus::opts!($NAME, $HELP))
    }};
}
