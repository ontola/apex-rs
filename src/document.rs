use crate::db_context::DbContext;
use crate::hashtuple::{HashModel, LookupTable};
use crate::models::*;
use crate::schema;
use diesel::insert_into;
use diesel::prelude::*;

pub(crate) fn get_doc_count(db_conn: &PgConnection) {
    use diesel::dsl;
    use schema::documents::dsl::*;
    use schema::properties::dsl::*;
    use schema::resources::dsl::*;

    let doc_count: i64 = documents.select(dsl::count_star()).first(db_conn).unwrap();
    let resource_count: i64 = resources.select(dsl::count_star()).first(db_conn).unwrap();
    let property_count: i64 = properties.select(dsl::count_star()).first(db_conn).unwrap();

    println!(
        "we got {:?} documents, {:?} resources and {:?} properties",
        doc_count, resource_count, property_count
    );
}

const EMPTY_STRING: &str = "";

pub(crate) fn reset_document<'a>(
    ctx: &DbContext,
    lookup_table: &'a mut LookupTable,
    id: i64,
) -> HashModel {
    let doc = get_document(ctx.db_conn, id);
    let first = doc.first();
    let mut props: HashModel = vec![];

    if doc.is_empty() {
        let doc = &Document {
            id,
            iri: format!("https://id.openraadsinformatie.nl/{}", id),
        };
        insert_into(crate::schema::documents::table)
            .values(doc)
            .execute(ctx.db_conn)
            .expect("Error while inserting into documents");
    } else {
        let (doc, resources) = first.unwrap();
        for (resource, properties) in resources {
            for p in properties {
                let predicate = ctx
                    .property_map
                    .iter()
                    .find(|(_, v)| **v == p.predicate_id)
                    .unwrap()
                    .0;
                let datatype = ctx
                    .datatype_map
                    .iter()
                    .find(|(_, v)| **v == p.datatype_id)
                    .unwrap()
                    .0;
                props.push([
                    lookup_table.ensure_value(&resource.iri),
                    lookup_table.ensure_value(predicate),
                    lookup_table.ensure_value(&p.value),
                    lookup_table.ensure_value(datatype),
                    lookup_table.ensure_value(&String::from(EMPTY_STRING)), // p.language.clone()
                    lookup_table.ensure_value(&String::from(EMPTY_STRING)),
                ]);
            }
        }
        println!(
            "Fetched document: {} with {} existing properties",
            doc.iri,
            props.len()
        );

        delete_document_data(ctx.db_conn, id)
    }

    props
}

fn delete_document_data(db_conn: &PgConnection, doc_id: i64) {
    use schema::properties;
    use schema::resources::dsl::*;

    println!("start delete");
    let resource_ids = resources
        .select(id)
        .filter(document_id.eq(doc_id))
        .get_results::<i64>(db_conn)
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

fn get_document(
    db_conn: &PgConnection,
    doc_id: i64,
) -> Vec<(Document, Vec<(Resource, Vec<Property>)>)> {
    use schema::documents::dsl::*;

    let docs: Vec<Document> = documents
        .filter(id.eq(doc_id))
        .load::<Document>(db_conn)
        .unwrap();

    let doc_resources: Vec<Resource> = Resource::belonging_to(&docs)
        .load::<Resource>(db_conn)
        .unwrap();

    let doc_properties: Vec<Property> =
        match Property::belonging_to(&doc_resources).load::<Property>(db_conn) {
            Ok(res) => res,
            Err(e) => {
                println!("{:?}", e);
                vec![]
            }
        };

    let grouped_properties: Vec<Vec<Property>> = doc_properties.grouped_by(&doc_resources);

    let resources_and_properties = doc_resources
        .into_iter()
        .zip(grouped_properties)
        .grouped_by(&docs);

    docs.into_iter().zip(resources_and_properties).collect()
}
