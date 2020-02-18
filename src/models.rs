use crate::schema::documents;
use crate::schema::properties;
use crate::schema::resources;

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable, Insertable)]
pub struct Document {
    pub id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
#[belongs_to(Document)]
pub struct Resource {
    pub id: i32,
    pub document_id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "resources"]
#[belongs_to(Document)]
pub struct NewResource {
    pub document_id: i32,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
#[table_name = "properties"]
#[primary_key(resource_id, predicate, order)]
#[belongs_to(Resource)]
pub struct Property {
    //    pub id: uuid::Uuid,
    pub resource_id: i32,
    pub predicate: String,
    pub order: Option<i32>,
    pub value: String,
    pub datatype: String,
    pub language: String,
    pub prop_resource: Option<i32>,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "properties"]
pub struct NewProperty {
    pub resource_id: i32,
    pub predicate: String,
    pub order: Option<i32>,
    pub value: String,
    pub datatype: String,
    pub language: String,
    pub prop_resource: Option<i32>,
}
