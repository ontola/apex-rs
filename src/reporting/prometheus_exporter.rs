// MIT License
//
// Copyright (c) [2019] [Alexander Thaller]
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#![deny(missing_docs)]

use hyper::{
    rt::{self, Future},
    service::service_fn_ok,
    Body, Error as HyperError, Method, Response, Server,
};
#[cfg(feature = "log")]
use log::{error, info};
use prometheus::{Encoder, TextEncoder};
use std::{error::Error, fmt, net::SocketAddr};

/// Errors that can happen when the prometheus exporter gets started.
#[derive(Debug)]
pub enum StartError {
    /// Hyper related errors.
    HyperError(HyperError),
}

impl From<HyperError> for StartError {
    fn from(err: HyperError) -> Self {
        StartError::HyperError(err)
    }
}

impl fmt::Display for StartError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for StartError {
    fn cause(&self) -> Option<&dyn Error> {
        match self {
            StartError::HyperError(err) => Some(err),
        }
    }
}

/// Struct that holds everything together.
pub struct PrometheusExporter;

impl PrometheusExporter {
    /// Start the prometheus exporter and bind the hyper http server to the
    /// given socket.
    pub fn run(addr: &SocketAddr) -> Result<(), StartError> {
        let service = move || {
            let encoder = TextEncoder::new();

            service_fn_ok(move |req| match (req.method(), req.uri().path()) {
                (&Method::GET, "/metrics") => PrometheusExporter::send_metrics(&encoder),
                _ => PrometheusExporter::send_redirect(),
            })
        };

        let server = Server::try_bind(&addr)?
            .serve(service)
            .map_err(log_serving_error);

        log_startup(&addr);

        rt::run(server);

        Ok(())
    }

    fn send_metrics(encoder: &TextEncoder) -> Response<Body> {
        let metric_families = prometheus::gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer).unwrap();

        Response::new(Body::from(buffer))
    }

    fn send_redirect() -> Response<Body> {
        let message = "try /metrics for metrics\n";
        Response::builder()
            .status(301)
            .body(Body::from(message))
            .unwrap()
    }
}

#[allow(unused)]
fn log_startup(addr: &SocketAddr) {
    #[cfg(feature = "log")]
    info!("Listening on http://{}", addr);
}

#[allow(unused)]
fn log_serving_error(error: HyperError) {
    #[cfg(feature = "log")]
    error!("problem while serving metrics: {}", error)
}
