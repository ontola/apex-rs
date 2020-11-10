use crate::errors::ErrorKind;
use crate::importing::events::MessageTiming;
use crate::reporting::prometheus_exporter::PrometheusExporter;
use crate::reporting::reporter::Reporter;
use std::borrow::BorrowMut;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::mpsc::*;
use tokio::task;

pub async fn report_prometheus(
    mut rx: &mut Receiver<Result<MessageTiming, ErrorKind>>,
) -> Result<((), ()), String> {
    println!("Reported started");
    let reporter = Reporter::default();
    let reporter = Arc::new(reporter);

    tokio::try_join!(open_server(), process_reports(&mut rx, reporter.clone()))
}

async fn open_server() -> Result<(), String> {
    info!(target: "apex", "Starting metrics server");
    // Parse address used to bind exporter to.
    let addr_raw = "0.0.0.0:3031";
    let addr: SocketAddr = addr_raw.parse().expect("can not parse listen addr");

    let result = task::spawn_blocking(move || {
        if let Err(e) = PrometheusExporter::run(&addr) {
            error!(target: "apex", "Error starting metrics server: {}", e);
        }
    })
    .await;

    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

async fn process_reports(
    rx: &mut Receiver<Result<MessageTiming, ErrorKind>>,
    mut reporter: Arc<Reporter>,
) -> Result<(), String> {
    debug!(target: "apex", "Started report processor");
    let reporter_updater = reporter.borrow_mut();

    loop {
        let msg = rx.recv().await.unwrap();
        debug!(target: "apex", "Receive msg");

        reporter_updater.update_processing_rate();
        match msg {
            Ok(timing) => reporter_updater.add_timing(timing),
            Err(e) => reporter_updater.add_error(e),
        }

        task::yield_now().await;
    }
}
