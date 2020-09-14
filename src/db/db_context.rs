use crate::db::models::{ConfigItem, Datatype, Language, Predicate};
use crate::db::schema;
use crate::db::schema::documents::dsl::documents;
use crate::db::schema::languages::dsl::languages;
use crate::hashtuple::LookupTable;
use bimap::{BiHashMap, BiMap};
use diesel::prelude::*;
use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{r2d2, PgConnection};
use std::env;
use std::hash::Hash;

pub type IRIMapping = BiMap<String, i32>;

pub struct DbContext<'a> {
    pub db_pool: &'a DbPool,
    pub config: Config,
    pub property_map: IRIMapping,
    pub datatype_map: IRIMapping,
    pub language_map: IRIMapping,
    pub resource_map: BiMap<String, i64>,
    pub lookup_table: LookupTable,
}

pub struct DbCounts {
    pub documents: i64,
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
            resource_map: BiMap::new(),
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
        DbContext::custom_pool(
            env::var("DATABASE_URL")
                .expect("No DATABASE_URL set")
                .as_str(),
        )
    }

    pub fn est_counts(&self) -> DbCounts {
        let conn = self.get_conn();

        let documents_est = self.count_table(&conn, "documents");

        DbCounts {
            documents: documents_est,
        }
    }

    fn count_table(&self, conn: &PgConnection, _todo_table_name: &str) -> i64 {
        match documents.select(diesel::dsl::count_star()).first(conn) {
            Ok(v) => v,
            Err(_) => -1,
        }
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

    let mut map = BiMap::new();
    let props = predicates
        .limit(100_000)
        .load::<Predicate>(&db_conn.get().unwrap())
        .expect("Could not fetch properties");

    for p in props {
        verified_ensure(
            &mut map,
            p.value.clone(),
            p.id,
            "Predicate or hash already present",
        );
    }

    map
}

/// Retrieve a map of data type IRIs to their ids from the db.
fn get_datatypes(db_conn: &DbPool) -> IRIMapping {
    use schema::datatypes::dsl::*;

    let mut map = BiMap::new();
    let props = datatypes
        .limit(100_000)
        .load::<Datatype>(&db_conn.get().unwrap())
        .expect("Could not fetch datatypes");

    for p in props {
        verified_ensure(
            &mut map,
            p.value.clone(),
            p.id,
            "Datatype or hash already present",
        );
    }

    map
}

/// Retrieve a map of language IRIs to their ids from the db.
fn get_languages(db_conn: &DbPool) -> IRIMapping {
    let mut map = BiMap::new();
    let props = languages
        .limit(100_000)
        .load::<Language>(&db_conn.get().unwrap())
        .expect("Could not fetch languages");

    for p in props {
        verified_ensure(
            &mut map,
            p.value.clone(),
            p.id,
            "Language or hash already present",
        );
    }

    map
}

pub(crate) fn verified_ensure<L, R>(map: &mut BiHashMap<L, R>, left: L, right: R, msg: &'static str)
where
    L: Eq + Hash + Clone,
    R: Copy + Eq + Hash,
{
    if let Err((left_err, right_err)) = map.insert_no_overwrite(left.clone(), right) {
        if left != left_err || right != right_err {
            panic!(msg);
        }
    }
}
