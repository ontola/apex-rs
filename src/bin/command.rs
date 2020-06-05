extern crate apex_rs;
extern crate dotenv;
extern crate log;

use apex_rs::importing::redis::{CHANNEL, create_redis_connection};
use apex_rs::writing::ndjson_serializer::serialize;
use std::env;
use redis::{self, Commands};

/// Simple CLI for editing data in Apex-RS. Publishes deltas to Redis.
fn main() {
  let args: Vec<String> = env::args().collect();
  println!("{:?}", args);
  let message = serialize();
  let mut con = create_redis_connection().expect("Connection to redis failed");
  let _: () = con.publish(CHANNEL, message).expect("Could not publish command to redis");
}
