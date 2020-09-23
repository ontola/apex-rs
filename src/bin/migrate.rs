extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel_migrations;

use apex_rs::app_config::AppConfig;
use apex_rs::db::db_context::DbContext;
use apex_rs::db::models::ConfigItem;
use apex_rs::db::schema;
use clap::{App, Arg};
use diesel::connection::SimpleConnection;
use diesel::result::Error::NotFound;
use diesel::{Connection, ExpressionMethods, PgConnection, QueryDsl, RunQueryDsl};
use dotenv::dotenv;
use url::Url;

embed_migrations!("./migrations");

/// Tool to run after running a migration which can't/is to bothersome to be expressed in SQL
fn main() -> Result<(), String> {
    env_logger::init();
    if cfg!(debug_assertions) {
        match dotenv() {
            Ok(_) => info!(target: "apex", "Initialized .env"),
            Err(e) => warn!(target: "apex", "Error loading .env: {}", e),
        }
    }

    let matches = App::new("Apex migrate")
        .version("1.0")
        .arg(
            Arg::with_name("version")
                .short("v")
                .long("version")
                .value_name("FILE")
                .help("The version of the migration code to run")
                .takes_value(true),
        )
        .get_matches();

    match matches.value_of("version") {
        Some("setup") => setup(),
        Some(_) => Err("Invalid version".into()),
        None => Err("Provide a version to run".into()),
    }
}

fn setup() -> Result<(), String> {
    use schema::_apex_config::dsl;

    info!(target: "apex", "Running setup");

    let config = AppConfig::default();
    let db_url = config
        .database_url
        .clone()
        .expect("No DB connection string");
    let mut connstr = Url::parse(&db_url).unwrap();
    connstr.set_path("");
    // Wait until db is up
    DbContext::default_pool(Some(connstr.to_string()))?;

    match PgConnection::establish(&connstr.to_string()) {
        Ok(c) => {
            let q = format!("CREATE DATABASE {}", config.database_name);
            if let Err(e) = c.batch_execute(&q) {
                warn!(target: "apex", "Error creating database: {}", e);
            } else {
                info!(target: "apex", "Created database {}", config.database_name);
            }
        }
        Err(e) => {
            error!(target: "apex", "Can't connect to db: {}", e);
        }
    }

    let pool = DbContext::default_pool(config.database_url.clone())?;

    match embedded_migrations::run_with_output(
        &pool.get().expect("Can't connect to db"),
        &mut std::io::stdout(),
    ) {
        Ok(_) => info!("Migrations succeeded"),
        Err(e) => error!("Migrations failed: {}", e),
    }

    let seed = dsl::_apex_config
        .filter(dsl::key.eq("seed"))
        .get_result::<ConfigItem>(&pool.get().unwrap());

    match seed {
        Ok(v) => {
            info!("Seed already present ({}), skipping", v.value);
            Ok(())
        }
        Err(NotFound) => {
            info!("Adding 'seed' config item");
            let value = rand::random::<u32>().to_string();
            diesel::insert_into(dsl::_apex_config)
                .values(ConfigItem {
                    key: "seed".into(),
                    value,
                })
                .execute(&pool.get().expect("Can't connect to db"))
                .expect("Setup seed failed");

            Ok(())
        }
        Err(e) => {
            error!(target: "apex", "Unexpected error occurred: {}", e);
            Err(e.to_string())
        }
    }
}
