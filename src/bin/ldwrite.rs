extern crate apex_rs;
extern crate dotenv;
extern crate log;

use apex_rs::importing::delta_processor::{LD_ADD, LD_REPLACE, LD_SUPPLANT};
use apex_rs::importing::redis::{create_redis_connection, CHANNEL};
use apex_rs::writing::ndjson_serializer::{serialize_hextuple_redis, Tuple};
use clap::{self, App, Arg};
use redis::{self, Commands};
use rio_api::parser::TriplesParser;
use rio_turtle::{TurtleError, TurtleParser};

static METHODS: &[&str] = &["add", "replace", "supplant"];

fn main() {
    // Use Clap only for providing some help functions.
    let matches = App::new("ldwrite")
        .version("0.1.1")
        .about("Simple CLI for editing data in Apex-RS. Publishes linked-deltas to Redis. Reads URI prefixes from ~/.ldget/prefixes.")
        .author("Joep Meindertsma - joep@ontola.io")
        .arg(
            Arg::with_name("method")
            .possible_values(METHODS)
            .required(true)
            .help("How the statement should be processed.")
        )
        .arg(
            Arg::with_name("subject")
            .required(true)
            .help("A subject URI, e.g. <https://example.com/something> or prefix:SomeThing")
        )
        .arg(
            Arg::with_name("predicate")
            .required(true)
            .help("A predicate URI, e.g. <https://example.com/something> or prefix:SomeThing")
        )
        .arg(
            Arg::with_name("object")
            .required(true)
            .help("An object URI or literal value , e.g. <https://example.com/something>, prefix:SomeThing or \\\"string value\\\"^xsd:string (escape quotes!) ")
        )
        .get_matches()
        ;
    let subject_arg = matches.value_of("subject").expect("No subject");
    let predicate_arg = matches.value_of("predicate").expect("No predicate");
    let object_arg = matches.value_of_lossy("object").expect("No object");
    let method;
    match matches.value_of("method").expect("no method") {
        "add" => method = LD_ADD,
        "replace" => method = LD_REPLACE,
        "supplant" => method = LD_SUPPLANT,
        _ => panic!(format!("Unknown command. {}", AVAILABLE_COMMANDS)),
    };

    // I use a Turtle parser to parse the arguments and apply the prefixes
    let mut ttl: String = String::from("");

    for prefix in get_prefixes() {
        ttl.push_str(format!("@prefix {}: <{}> .\n", prefix.key, prefix.value).as_str())
    }

    ttl.push_str(format!("{} {} {} .\n", subject_arg, predicate_arg, object_arg).as_str());
    let mut subject: String = String::from("");
    let mut predicate: String = String::from("");
    let mut val: String = String::from("");
    let mut dt: String = String::from("");
    let mut lang: String = String::from("");
    TurtleParser::new(ttl.as_ref(), "")
        .expect("Failed to start parser")
        .parse_all(&mut |trip| {
            match trip.subject {
                rio_api::model::NamedOrBlankNode::NamedNode(nn) => subject = nn.iri.into(),
                rio_api::model::NamedOrBlankNode::BlankNode(bn) => subject = bn.id.into(),
            };

            predicate = trip.predicate.iri.into();
            match trip.object {
                rio_api::model::Term::NamedNode(nn) => val = nn.iri.into(),
                rio_api::model::Term::BlankNode(bn) => val = bn.id.into(),
                rio_api::model::Term::Literal(li) => match li {
                    rio_api::model::Literal::Simple { value } => {
                        val = value.into();
                    }
                    rio_api::model::Literal::LanguageTaggedString { value, language } => {
                        val = value.into();
                        lang = language.into();
                        dt = "http://www.w3.org/2001/XMLSchema#string".into();
                    }
                    rio_api::model::Literal::Typed { value, datatype } => {
                        val = value.into();
                        dt = datatype.iri.into();
                    }
                },
            }
            Ok(()) as Result<(), TurtleError>
        })
        .expect("Could not parse input as Turtle");

    let tuple = Tuple::new(subject, predicate, val, dt, lang, String::from(method));

    let message = serialize_hextuple_redis(tuple);
    let mut con = create_redis_connection().expect("Connection to redis failed");
    let _: () = con
        .publish(CHANNEL, &message)
        .expect("Could not publish command to redis");
    println!("Published to redis: {:?}", message);
}

const AVAILABLE_COMMANDS: &str = "Available: 'add', 'replace', 'supplant'";
/// A single key / value combination for URL shorthands
#[derive(Debug)]
struct Prefix {
    /// The shorthand (e.g. 'foaf')
    key: String,
    /// The base URL (e.g. 'http://xmlns.com/foaf/0.1/')
    value: String,
}

/// Finds the prefixes file, parses it and returns the Prefixes
fn get_prefixes() -> Vec<Prefix> {
    let mut prefixes: Vec<Prefix> = Vec::new();

    let mut prefixes_path = dirs::home_dir().expect("No home dir found.");
    prefixes_path.push(".ldget/prefixes");

    let foaf_prefix = Prefix {
        key: String::from("foaf"),
        value: String::from("http://xmlns.com/foaf/0.1/"),
    };
    let rdf_prefix = Prefix {
        key: String::from("rdf"),
        value: String::from("http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    };

    prefixes.push(foaf_prefix);
    prefixes.push(rdf_prefix);

    match std::fs::read_to_string(prefixes_path) {
        Ok(contents) => {
            for line in contents.lines() {
                match line.chars().next() {
                    Some('#') => {}
                    Some(' ') => {}
                    Some(_) => {
                        let split: Vec<&str> = line.split("=").collect();
                        if split.len() == 2 {
                            let found = Prefix {
                                key: String::from(split[0]),
                                value: String::from(split[1]),
                            };
                            prefixes.push(found)
                        };
                    }
                    None => {}
                };
            }
            prefixes
        }
        Err(_) => prefixes
    }
}
