extern crate apex_rs;
extern crate dotenv;
extern crate log;

use apex_rs::importing::delta_processor::{LD_ADD, LD_REPLACE, LD_SUPPLANT};
use apex_rs::importing::redis::{create_redis_connection, CHANNEL};
use apex_rs::writing::ndjson_serializer::{serialize_hextuple_redis, Tuple};
use redis::{self, Commands};

use rio_api::{model::Triple, parser::TriplesParser};
use rio_turtle::{TurtleError, TurtleParser};

const AVAILABLE_COMMANDS: &str = "Available: 'add', 'replace', 'supplant'";

const TTL_BASE: &str = "
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .

rdf:joep foaf:test \"someval\"@en-US.
";

/// Simple CLI for editing data in Apex-RS. Publishes deltas to Redis.
fn main() {
    let method_arg = std::env::args()
        .nth(1)
        .expect(format!("No method given. {}", AVAILABLE_COMMANDS).as_str());
    let subject_arg = std::env::args().nth(2).expect("No subject given.");
    let predicate_arg = std::env::args().nth(3).expect("No predicate given.");
    let object_arg = std::env::args().nth(3).expect("No object given.");
    let mut method = "";
    match method_arg.as_str() {
        "add" => method = LD_ADD,
        "replace" => method = LD_REPLACE,
        "supplant" => method = LD_SUPPLANT,
        _ => panic!(format!("Unknown command. {}", AVAILABLE_COMMANDS)),
    };

    let mut subject: String = String::from("");
    let mut predicate: String = String::from("");
    let mut val: String = String::from("");
    let mut dt: String = String::from("");
    let mut lang: String = String::from("");
    TurtleParser::new(TTL_BASE.as_ref(), "")
        .unwrap()
        .parse_all(&mut |trip| {
            match trip.subject {
                rio_api::model::NamedOrBlankNode::NamedNode(nn) => subject = nn.iri.into(),
                rio_api::model::NamedOrBlankNode::BlankNode(bn) => subject = bn.id.into(),
            };

            predicate = trip.predicate.iri.into();
            match trip.object {
                rio_api::model::Term::NamedNode(nn) => val = nn.iri.into(),
                rio_api::model::Term::BlankNode(bn) => val = bn.id.into(),
                rio_api::model::Term::Literal(li) => {
                    match li {
                        rio_api::model::Literal::Simple { value } => {
                            val = value.into();
                        }
                        rio_api::model::Literal::LanguageTaggedString { value, language } => {
                            val = value.into();
                            lang = language.into();
                        }
                        rio_api::model::Literal::Typed { value, datatype } => {
                            val = value.into();
                            dt = datatype.iri.into();
                        }
                    }
                }
            }
            Ok(()) as Result<(), TurtleError>
        })
        .unwrap();

    let tuple = Tuple::new(subject, predicate, val, dt, lang, String::from(method));

    let message = serialize_hextuple_redis(tuple);
    println!("{:?}", message);
    let mut con = create_redis_connection().expect("Connection to redis failed");
    let _: () = con
        .publish(CHANNEL, message)
        .expect("Could not publish command to redis");
}
