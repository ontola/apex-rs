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
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::{insert_into, sql_query};
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

    let mut property_map = get_predicates(&db_conn);
    let datatype_map = get_datatypes(&db_conn);

    while let Some(message) = stream.next().await {
        match message {
            Err(e) => {
                warn!("Kafka error: {}", e);
            }
            Ok(m) => {
                let transaction = db_conn.transaction::<(), diesel::result::Error, _>(|| {
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

                        let existing = reset_document(&db_conn, &property_map, &datatype_map, id);
                        fetch_time += SystemTime::now()
                            .duration_since(fetch_start)
                            .unwrap()
                            .as_millis();

                        let next = apply_delta(&existing, &delta);

                        //// Replace
                        let insert_start = SystemTime::now();
                        let resources = insert_resources(&db_conn, &next, id);
                        let mut resource_id_map = HashMap::<String, i32>::new();
                        for resource in resources {
                            resource_id_map.insert(resource.iri.clone(), resource.id);
                        }

                        insert_properties(
                            &db_conn,
                            &mut property_map,
                            &datatype_map,
                            &next,
                            resource_id_map,
                        );
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

fn get_datatypes(db_conn: &PgConnection) -> HashMap<String, i32> {
    use schema::datatypes::dsl::*;

    let mut map = HashMap::new();
    let props = datatypes
        .limit(100_000)
        .load::<Datatype>(db_conn)
        .expect("Could not fetch datatypes");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}

fn get_predicates(db_conn: &PgConnection) -> HashMap<String, i32> {
    use schema::predicates::dsl::*;

    let mut map = HashMap::new();
    let props = predicates
        .limit(100_000)
        .load::<Predicate>(db_conn)
        .expect("Could not fetch properties");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}

fn delete_document_data(db_conn: &PgConnection, doc_id: i32) {
    use schema::properties;
    use schema::resources::dsl::*;

    println!("start delete");
    let resource_ids = resources
        .select(id)
        .filter(document_id.eq(doc_id))
        .get_results::<i32>(db_conn)
        .expect("Could not fetch resource ids for document");

    let props = properties::dsl::resource_id.eq_any(&resource_ids);
    diesel::delete(properties::table)
        .filter(props)
        .execute(db_conn)
        .expect("Couldn't delete existing properties");

    diesel::delete(resources)
        .filter(id.eq_any(&resource_ids))
        .execute(db_conn)
        .expect("Couldn't delete existing resources");

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

fn insert_properties(
    db_conn: &PgConnection,
    mut predicate_map: &mut HashMap<String, i32>,
    datatype_map: &HashMap<String, i32>,
    model: &Model,
    resource_id_map: HashMap<String, i32>,
) {
    use crate::schema::properties::dsl;

    let mut properties = vec![];

    for h in model {
        let resource_id = *resource_id_map
            .get(&h[0])
            .expect("Inserting property not inserted in resources");

        if !predicate_map.contains_key(&h[1]) {
            insert_and_update(db_conn, &mut predicate_map, &h[1]);
        }

        let pred_id: i32 = *predicate_map.get_mut(&h[1]).unwrap();

        properties.push((
            dsl::resource_id.eq(resource_id),
            dsl::predicate_id.eq(pred_id),
            //            dsl::order.eq(None),
            dsl::value.eq(String::from(&h[2])),
            dsl::datatype_id.eq(datatype_map
                .get(&h[3])
                .unwrap_or_else(|| panic!("Datatype not found in map ({})", &h[3]))),
            //            dsl::language_id.eq(Some(0)),
            //            dsl::prop_resource.eq(None),
        ));
    }

    insert_into(self::schema::properties::table)
        .values(&properties)
        .execute(db_conn)
        .expect("Error while inserting into resources");
}

fn insert_and_update(
    db_conn: &PgConnection,
    predicate_map: &mut HashMap<String, i32>,
    predicate_value: &str,
) -> i32 {
    use schema::predicates::dsl::*;

    let target = value.eq(predicate_value);
    let p = insert_into(predicates)
        .values(vec![(&target)])
        .get_result::<Predicate>(db_conn)
        .unwrap_or_else(|_| {
            predicates
                .filter(&target)
                .get_result(db_conn)
                .unwrap_or_else(|_| panic!("Predicate not found {}", predicate_value))
        });
    predicate_map.entry(p.value).or_insert(p.id);

    p.id
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

fn reset_document(
    db_conn: &PgConnection,
    property_map: &HashMap<String, i32>,
    datatype_map: &HashMap<String, i32>,
    id: i32,
) -> Model {
    let doc = get_document(db_conn, id);
    let first = doc.first();
    let mut props: Vec<Hextuple> = vec![];

    if doc.is_empty() {
        let doc = &Document {
            id,
            iri: format!("https://id.openraadsinformatie.nl/{}", id),
        };
        insert_into(self::schema::documents::table)
            .values(doc)
            .execute(db_conn)
            .expect("Error while inserting into documents");
    } else {
        let (doc, resources) = first.unwrap();
        for (resource, properties) in resources {
            for p in properties {
                let predicate = property_map
                    .iter()
                    .find(|(k, v)| **v == p.predicate_id)
                    .unwrap()
                    .0;
                let datatype = datatype_map
                    .iter()
                    .find(|(k, v)| **v == p.datatype_id)
                    .unwrap()
                    .0;
                props.push([
                    resource.iri.clone(),
                    predicate.to_string(),
                    p.value.clone(),
                    datatype.to_string(),
                    "".into(), // p.language.clone(),
                    "".to_string(),
                ]);
            }
        }
        println!(
            "Fetched document: {} with {} existing properties",
            doc.iri,
            props.len()
        );

        delete_document_data(&db_conn, id)
    }

    props
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

    map.entry(String::from(id)).or_insert_with(|| vec![]);

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
