#[macro_use]
extern crate log;
extern crate dotenv;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate dotenv_codegen;

mod delta;
mod models;
mod schema;
mod types;

use crate::delta::apply_delta;
use crate::models::*;
use diesel::insert_into;
use diesel::prelude::*;
use dotenv::dotenv;
use futures::StreamExt;
use rdkafka::config::RDKafkaLogLevel;
use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use rio_api::model::{Literal, NamedOrBlankNode, Term};
use rio_api::parser::QuadsParser;
use rio_turtle::{NQuadsParser, TurtleError};
use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::time::SystemTime;
use std::{io, str};
use types::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    println!("Hello, world!");
    let mut config = ClientConfig::new();
    config.set("bootstrap.servers", dotenv!("KAFKA_ADDRESS"));

    config.set("sasl.mechanisms", "PLAIN");
    config.set("security.protocol", "SASL_SSL");
    config.set("sasl.username", dotenv!("KAFKA_USERNAME"));
    config.set("sasl.password", dotenv!("KAFKA_PASSWORD"));

    config.set("group.id", dotenv!("KAFKA_GROUP_ID"));
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
    let topic = dotenv!("KAFKA_TOPIC");

    let consumer = config.create::<StreamConsumer>()?;

    consumer.subscribe(&[topic])?;

    let db_conn = PgConnection::establish(dotenv!("DATABASE_URL"))
        .expect(&"Error connecting to pg".to_string());

    //    get_doc_count(db_conn);

    let mut stream = consumer.start();
    let mut i = 0i64;
    let mut parse_time = 0u128;
    let mut fetch_time = 0u128;
    let mut insert_time = 0u128;

    while let Some(message) = stream.next().await {
        match message {
            Err(e) => {
                warn!("Kafka error: {}", e);
            }
            Ok(m) => {
                let transaction = db_conn.transaction::<(), diesel::result::Error, _>(|| {
                    //                let _key = str::from_utf8(&m.key().expect("message has no key")).unwrap();
                    let payload = &m.payload().expect("message has no payload");
                    let parse_start = SystemTime::now();
                    let docs = parse(payload);
                    parse_time += SystemTime::now()
                        .duration_since(parse_start)
                        .unwrap()
                        .as_millis();

                    for (k, delta) in docs {
                        let fetch_start = SystemTime::now();
                        let id = k.parse::<i32>().unwrap();
                        let existing = match existing_triples(&db_conn, id) {
                            Some(data) => {
                                delete_document(&db_conn, id);
                                data
                            }
                            None => vec![],
                        };
                        fetch_time += SystemTime::now()
                            .duration_since(fetch_start)
                            .unwrap()
                            .as_millis();
                        let next = apply_delta(&existing, delta);

                        //// Replace
                        // TODO: Delete existing data
                        let doc = &Document {
                            id,
                            iri: format!("https://id.openraadsinformatie.nl/{}", id),
                        };
                        let insert_start = SystemTime::now();
                        insert_into(self::schema::documents::table)
                            .values(doc)
                            .execute(&db_conn)
                            .expect("Error while inserting into documents");

                        let resources = insert_resources(&db_conn, &next, id);
                        let mut resource_id_map = HashMap::<String, i32>::new();
                        for resource in resources {
                            resource_id_map.insert(resource.iri.clone(), resource.id);
                        }

                        insert_properties(&db_conn, &next, resource_id_map);
                        insert_time += SystemTime::now()
                            .duration_since(insert_start)
                            .unwrap()
                            .as_millis();
                    }

                    // Now that the message is completely processed, add it's position to the offset
                    // store. The actual offset will be committed every 5 seconds.
                    if let Err(e) = consumer.store_offset(&m) {
                        warn!("Error while storing offset: {}", e);
                    }

                    Ok(())
                });

                match transaction {
                    Ok(_) => print!("."),
                    Err(_) => print!("e"),
                };
                i += 1;
                if i > 5 {
                    println!(
                        "\nTiming: parse: {}, fetch/del: {}, insert: {}",
                        parse_time, fetch_time, insert_time
                    );
                    parse_time = 0;
                    fetch_time = 0;
                    insert_time = 0;

                    io::stdout().flush().expect("Flush to stdout failed");
                    i = 0;
                }
            }
        }
    }

    Ok(())
}

fn delete_document(db_conn: &PgConnection, doc_id: i32) {
    use schema::documents::dsl::*;

    println!("start delete");
    let test = id.eq(doc_id);
    diesel::delete(documents.filter(test))
        .execute(db_conn)
        .expect("Tried to delete nonexisting document");
    println!("deleted resource");
}

fn insert_resources(db_conn: &PgConnection, model: &Model, id: i32) -> Vec<Resource> {
    let mut resource_iris = HashSet::new();
    for hex in model {
        resource_iris.insert(hex.get(0).unwrap().clone());
    }
    let mut new_resources = vec![];
    for r_iri in resource_iris {
        new_resources.push(NewResource {
            document_id: id,
            iri: r_iri,
        })
    }

    insert_into(self::schema::resources::table)
        .values(&new_resources)
        .get_results(db_conn)
        .expect("Error while inserting into resources")
}

fn insert_properties(db_conn: &PgConnection, model: &Model, resource_id_map: HashMap<String, i32>) {
    let mut properties = vec![];

    for h in model {
        let resource_id = *resource_id_map
            .get(&h[0])
            .expect("Inserting property not inserted in resources");

        properties.push(NewProperty {
            resource_id,
            predicate: String::from(&h[1]),
            order: None,
            value: String::from(&h[2]),
            datatype: String::from(&h[3]),
            language: String::from(&h[4]),
            prop_resource: None,
        });
    }

    insert_into(self::schema::properties::table)
        .values(&properties)
        .execute(db_conn)
        .expect("Error while inserting into resources");
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

    let properties: Vec<Property> =
        match Property::belonging_to(&resources).load::<Property>(db_conn) {
            Ok(res) => res,
            Err(e) => {
                println!("{:?}", e);
                vec![]
            }
        };

    let grouped_properties: Vec<Vec<Property>> = properties.grouped_by(&resources);

    let resources_and_properties = resources
        .into_iter()
        .zip(grouped_properties)
        .grouped_by(&docs);

    docs.into_iter().zip(resources_and_properties).collect()
}

fn existing_triples(db_conn: &PgConnection, id: i32) -> Option<Model> {
    let doc = get_document(db_conn, id);
    let first = doc.first();

    if doc.is_empty() {
        None
    } else {
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

        Some(props)
    }
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
        .split('/')
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
        (*delta_op).to_string(),
    ]);
}

fn str_from_iri_or_bn(t: &NamedOrBlankNode) -> String {
    match t {
        NamedOrBlankNode::BlankNode(bn) => String::from(bn.id),
        NamedOrBlankNode::NamedNode(nn) => String::from(nn.iri),
    }
}

fn str_from_term(t: Term) -> [String; 3] {
    match t {
        Term::BlankNode(bn) => [
            String::from(bn.id),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#blankNode".into(),
            "".into(),
        ],
        Term::NamedNode(nn) => [
            String::from(nn.iri),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#namedNode".into(),
            "".into(),
        ],
        Term::Literal(Literal::Simple { value }) => [
            value.into(),
            "http://www.w3.org/2001/XMLSchema#string".into(),
            "".into(),
        ],
        Term::Literal(Literal::LanguageTaggedString { value, language }) => [
            value.into(),
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".into(),
            language.into(),
        ],
        Term::Literal(Literal::Typed { value, datatype }) => {
            [value.into(), datatype.iri.into(), "".into()]
        }
    }
}
