use crate::db::schema::*;
use crate::db::uu128::Uu128;
use chrono::NaiveDateTime;
use diesel::sql_types::*;

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Insertable, QueryableByName)]
#[table_name = "_apex_config"]
pub struct ConfigItem {
    #[sql_type = "VarChar"]
    pub key: String,
    #[sql_type = "VarChar"]
    pub value: String,
}

#[derive(
    Clone, Eq, PartialEq, Debug, Queryable, Associations, Identifiable, Insertable, QueryableByName,
)]
pub struct Document {
    #[sql_type = "Int8"]
    pub id: i64,
    #[sql_type = "VarChar"]
    pub iri: String,
    #[sql_type = "Timestamp"]
    pub created_at: NaiveDateTime,
    #[sql_type = "Timestamp"]
    pub updated_at: NaiveDateTime,
    #[sql_type = "SmallInt"]
    pub cache_control: i16,
    #[sql_type = "VarChar"]
    pub language: String,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "documents"]
pub struct NewDocument {
    pub iri: String,
    pub language: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
#[belongs_to(Document)]
pub struct Resource {
    pub id: i64,
    pub document_id: i64,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "resources"]
#[belongs_to(Document)]
pub struct NewResource {
    pub document_id: i64,
    pub iri: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
pub struct Predicate {
    pub id: i32,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
pub struct Datatype {
    pub id: i32,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
pub struct Language {
    pub id: i32,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Insertable, Hash)]
#[table_name = "objects"]
pub struct Object {
    pub hash: Uu128,
    pub value: String,
}

#[derive(Eq, PartialEq, Debug, Queryable, Associations, Identifiable)]
#[table_name = "properties"]
#[primary_key(resource_id, predicate_id, order)]
#[belongs_to(Resource)]
pub struct Property {
    pub id: i64,
    pub resource_id: i64,
    pub predicate_id: i32,
    pub order: i32,
    pub prop_resource: Option<i64>,
    pub datatype_id: i32,
    pub language_id: Option<i32>,
    pub value: String,
    pub object_id: Option<Uu128>,
}

#[derive(Eq, PartialEq, Debug, Associations, Insertable)]
#[table_name = "properties"]
pub struct NewProperty {
    pub resource_id: i64,
    pub predicate_id: i32,
    pub order: Option<i32>,
    pub value: String,
    pub datatype_id: i32,
    pub language_id: Option<i32>,
    pub prop_resource: Option<i64>,
}
