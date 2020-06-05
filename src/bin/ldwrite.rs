extern crate apex_rs;
extern crate dotenv;
extern crate log;

use apex_rs::importing::redis::{CHANNEL, create_redis_connection};
use apex_rs::writing::ndjson_serializer::{Tuple, serialize_hextuple_redis};
use apex_rs::importing::delta_processor::{LD_REPLACE, LD_ADD, LD_SUPPLANT};
use std::env;
use redis::{self, Commands};

/// Simple CLI for editing data in Apex-RS. Publishes deltas to Redis.
fn main() {
  let args: Vec<String> = env::args().collect();
  let method_arg: &str = match args[1].as_str() {
    "add" => LD_ADD,
    "replace" => LD_REPLACE,
    "supplant" => LD_SUPPLANT,
    _ => "nope",
  };

  let subject = "http://localhost:8080/test";
  let predicate = "http://schema.org/awdad";
  let value = "30000-06-08";
  let datatype = "http://www.w3.org/2001/XMLSchema#date";
  let language = "";
  let graph = method_arg;

  let tuple = Tuple::new(subject, predicate, value, datatype, language, graph);

  let message = serialize_hextuple_redis(tuple);
  println!("{:?}", message);
  let mut con = create_redis_connection().expect("Connection to redis failed");
  let _: () = con.publish(CHANNEL, message).expect("Could not publish command to redis");
}
