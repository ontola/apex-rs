#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel;

mod delta;
mod models;
mod schema;
mod types;

use crate::delta::apply_delta;
use crate::models::*;
use diesel::prelude::*;
use futures::StreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use rio_api::model::{Literal, NamedOrBlankNode, Term};
use rio_api::parser::QuadsParser;
use rio_turtle::{NQuadsParser, TurtleError};
use std::collections::HashMap;
use std::str;
use types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Hello, world!");
    let mut config = ClientConfig::new();
    config.set(
        "bootstrap.servers",
        "pkc-lzgmd.europe-west3.gcp.confluent.cloud",
    );

    config.set(
        "bootstrap.servers",
        "pkc-lzgmd.europe-west3.gcp.confluent.cloud:9092",
    );
    config.set("sasl.mechanisms", "PLAIN");
    config.set("security.protocol", "SASL_SSL");
    config.set("sasl.username", "");
    config.set("sasl.password", "");

    config.set("group.id", "archer_dev");
    config.set("queue.buffering.max.ms", "0");
    //    config.set("allow.auto.create.topics", "false");
    config.set("fetch.max.bytes", "5428800");
    //    config.set("fetch.max.wait.ms", "1000");
    //    config.set("max.poll.records", "500");
    config.set("session.timeout.ms", "60000");
    config.set("enable.auto.commit", "true");
    config.set("auto.commit.interval.ms", "1000");
    config.set("request.timeout.ms", "20000");
    config.set("retry.backoff.ms", "500");

    config.set_log_level(RDKafkaLogLevel::Debug);
    println!("Config initialized");
    let topic = "ori-delta";

    let consumer = config.create::<StreamConsumer>()?;

    consumer.subscribe(&vec![topic.as_ref()])?;

    let db_conn =
        &PgConnection::establish("postgres://ori_api_user:@34.90.249.196/ori_api?sslmode=require")
            .expect(&format!("Error connecting to pg"));

    get_doc_count(db_conn);

    let mut stream = consumer.start();

    while let Some(message) = stream.next().await {
        match message {
            Err(e) => {
                warn!("Kafka error: {}", e);
            }
            Ok(m) => {
                let key = str::from_utf8(&m.key().expect("message has no key")).unwrap();
                let payload = &m.payload().expect("message has no payload");
                println!("key: {}", &key);
                let docs = parse(payload);
                for (k, delta) in docs {
                    let id = k.parse::<i32>().unwrap();
                    let existing = existing_triples(db_conn, id);
                    let next = apply_delta(existing, delta);
                    //// Replace
                }

                // Now that the message is completely processed, add it's position to the offset
                // store. The actual offset will be committed every 5 seconds.
                if let Err(e) = consumer.store_offset(&m) {
                    warn!("Error while storing offset: {}", e);
                }
            }
        }
    }

    Ok(())
}

fn get_doc_count(db_conn: &PgConnection) {
    use self::schema::documents::dsl::*;
    use self::schema::properties::dsl::*;
    use self::schema::resources::dsl::*;
    use diesel::dsl;

    let doc_count: i64 = documents.select(dsl::count_star()).first(db_conn).unwrap();
    let resource_count: i64 = resources.select(dsl::count_star()).first(db_conn).unwrap();
    let property_count: i64 = properties.select(dsl::count_star()).first(db_conn).unwrap();

    println!(
        "we got {:?} documents, {:?} resources and {:?} properties",
        doc_count, resource_count, property_count
    );
}

type DieselResult<T> = Result<T, diesel::result::Error>;

fn get_document(
    db_conn: &PgConnection,
    doc_id: i32,
) -> Vec<(Document, Vec<(Resource, Vec<Property>)>)> {
    let docs: Vec<Document> = self::schema::documents::table
        .filter(self::schema::documents::dsl::id.eq(doc_id))
        .load::<Document>(db_conn)
        .unwrap();

    let resources: Vec<Resource> = Resource::belonging_to(&docs)
        .load::<Resource>(db_conn)
        .unwrap();

    let properties: Vec<Property> = Property::belonging_to(&resources)
        .load::<Property>(db_conn)
        .unwrap();

    let grouped_properties: Vec<Vec<Property>> = properties.grouped_by(&resources);

    let resources_and_properties = resources
        .into_iter()
        .zip(grouped_properties)
        .grouped_by(&docs);

    return docs.into_iter().zip(resources_and_properties).collect();
}

fn existing_triples(db_conn: &PgConnection, id: i32) -> Model {
    let doc = get_document(db_conn, id);
    let first = doc.first();

    return if doc.len() > 0 {
        let mut props: Vec<Hextuple> = vec![];
        let (doc, resources) = first.unwrap();
        for (resource, properties) in resources {
            for p in properties {
                props.push([
                    resource.iri.clone(),
                    p.predicate.clone(),
                    p.value.clone(),
                    p.datatype.clone(),
                    p.language.clone(),
                    "".to_string(),
                ]);
            }
        }
        println!(
            "Fetched document: {} with {} existing properties",
            doc.iri,
            props.len()
        );

        props
    // TODO: (fetch &) convert properties
    } else {
        println!("Processing new document: {}", id);
        vec![]
    };
}

fn parse(payload: &[u8]) -> HashMap<String, Vec<Hextuple>> {
    let mut docs: HashMap<String, Vec<Hextuple>> = HashMap::new();

    NQuadsParser::new(payload)
        .unwrap()
        .parse_all(&mut |q| {
            let subj = str_from_iri_or_bn(&q.subject);
            let pred = String::from(q.predicate.iri);
            let graph = str_from_iri_or_bn(&q.graph_name.unwrap());

            test(&mut docs, subj, pred, str_from_term(q.object), graph);
            Ok(()) as Result<(), TurtleError>
        })
        .unwrap();

    docs
}

fn test(
    map: &mut HashMap<String, Vec<Hextuple>>,
    subj: String,
    pred: String,
    obj: [String; 3],
    graph: String,
) {
    let test: Vec<&str> = graph.split("?graph=").collect();
    let delta_op = test.first().unwrap();
    let id = test
        .last()
        .unwrap()
        .split("/")
        .last()
        .expect("Graph not properly formatted");
    if !map.contains_key(id) {
        map.insert(id.into(), vec![]);
    }

    map.get_mut(id).unwrap().push([
        subj,
        pred,
        obj[0].clone(),
        obj[1].clone(),
        obj[2].clone(),
        delta_op.to_string(),
    ]);
}

fn str_from_iri_or_bn(t: &NamedOrBlankNode) -> String {
    return match t {
        NamedOrBlankNode::BlankNode(bn) => String::from(bn.id),
        NamedOrBlankNode::NamedNode(nn) => String::from(nn.iri),
    };
}

fn str_from_term(t: Term) -> [String; 3] {
    return match t {
        Term::BlankNode(bn) => [String::from(bn.id), "".into(), "".into()],
        Term::NamedNode(nn) => [String::from(nn.iri), "".into(), "".into()],
        Term::Literal(Literal::Simple { value }) => [value.into(), "".into(), "".into()],
        Term::Literal(Literal::LanguageTaggedString { value, language }) => {
            [value.into(), "".into(), language.into()]
        }
        Term::Literal(Literal::Typed { value, datatype }) => {
            [value.into(), datatype.iri.into(), "".into()]
        }
    };
}
