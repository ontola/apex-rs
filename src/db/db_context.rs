use crate::db::models::{ConfigItem, Datatype, Language, Predicate};
use crate::db::schema;
use crate::hashtuple::LookupTable;
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{r2d2, PgConnection};
use std::collections::HashMap;
use std::env;

pub type IRIMapping = HashMap<String, i32>;

pub struct DbContext<'a> {
    pub db_pool: &'a DbPool,
    pub config: Config,
    pub property_map: IRIMapping,
    pub datatype_map: IRIMapping,
    pub language_map: IRIMapping,
    pub lookup_table: LookupTable,
}

pub struct Config {
    /// The murmur seed used in this schema
    pub seed: u32,
}

pub type DbPool = Pool<ConnectionManager<PgConnection>>;

impl<'a> DbContext<'a> {
    pub fn get_conn(&self) -> PooledConnection<ConnectionManager<PgConnection>> {
        self.db_pool
            .get()
            .expect("Failed to get connection from pool")
    }

    pub fn new(db_pool: &DbPool) -> DbContext {
        let config = get_config(&db_pool).unwrap();

        DbContext {
            db_pool: &db_pool,
            property_map: get_predicates(&db_pool),
            datatype_map: get_datatypes(&db_pool),
            language_map: get_languages(&db_pool),
            lookup_table: LookupTable::new(config.seed),
            config,
        }
    }

    pub(crate) fn custom_pool(connspec: &str) -> DbPool {
        let manager = ConnectionManager::<PgConnection>::new(connspec);

        r2d2::Pool::builder()
            .build(manager)
            .expect("Failed to create pool.")
    }

    pub fn default_pool() -> DbPool {
        DbContext::custom_pool(env::var("DATABASE_URL").unwrap().as_str())
    }
}

/// Parses the _apex_config table into a config object.
pub(crate) fn get_config(db_conn: &DbPool) -> Result<Config, ()> {
    use schema::_apex_config::dsl;

    let seed = dsl::_apex_config
        .filter(dsl::key.eq("seed"))
        .load::<ConfigItem>(&db_conn.get().unwrap())
        .unwrap()
        .first()
        .expect("Config has no seed row")
        .value
        .parse::<u32>()
        .expect("Seed value isn't a valid integer");

    Ok(Config { seed })
}

/// Retrieve a map of predicate IRIs to their ids from the db.
fn get_predicates(db_conn: &DbPool) -> IRIMapping {
    use schema::predicates::dsl::*;

    let mut map = HashMap::new();
    let props = predicates
        .limit(100_000)
        .load::<Predicate>(&db_conn.get().unwrap())
        .expect("Could not fetch properties");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}

/// Retrieve a map of data type IRIs to their ids from the db.
fn get_datatypes(db_conn: &DbPool) -> IRIMapping {
    use schema::datatypes::dsl::*;

    let mut map = HashMap::new();
    let props = datatypes
        .limit(100_000)
        .load::<Datatype>(&db_conn.get().unwrap())
        .expect("Could not fetch datatypes");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}

/// Retrieve a map of language IRIs to their ids from the db.
fn get_languages(db_conn: &DbPool) -> IRIMapping {
    use schema::datatypes::dsl::*;

    let mut map = HashMap::new();
    let props = datatypes
        .limit(100_000)
        .load::<Language>(&db_conn.get().unwrap())
        .expect("Could not fetch languages");

    for p in props {
        map.entry(p.value.clone()).or_insert(p.id);
    }

    map
}
