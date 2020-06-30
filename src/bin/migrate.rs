extern crate apex_rs;
extern crate dotenv;
#[macro_use]
extern crate log;
#[macro_use]
extern crate diesel_migrations;

use apex_rs::db::db_context::DbContext;
use apex_rs::db::models::{ConfigItem, Object, Property};
use apex_rs::db::schema;
use apex_rs::db::uu128::Uu128;
use clap::{App, Arg};
use diesel::result::Error::RollbackTransaction;
use diesel::{Connection, ExpressionMethods, QueryDsl, RunQueryDsl};

embed_migrations!("./migrations");

/// Tool to run after running a migration which can't/is to bothersome to be expressed in SQL
fn main() {
    env_logger::init();

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
        Some("setup") => {
            setup();
        }
        Some("2020_05_15_152936") => {
            migrate_2020_05_15_152936();
        }
        Some(_) => println!("Invalid version"),
        None => println!("Provide a version to run"),
    };
}

fn setup() {
    use schema::_apex_config::dsl;

    info!("Running setup");

    let pool = DbContext::default_pool();

    match embedded_migrations::run_with_output(
        &pool.get().expect("Can't connect to db"),
        &mut std::io::stdout(),
    ) {
        Ok(_) => info!("Migrations succeeded"),
        Err(e) => error!("Migrations failed: {}", e),
    }

    let seed = dsl::_apex_config
        .filter(dsl::key.eq("seed"))
        .load::<ConfigItem>(&pool.get().unwrap());

    match seed {
        Err(_) => {
            info!("Adding 'seed' config item");
            let value = rand::random::<u32>().to_string();
            diesel::insert_into(dsl::_apex_config)
                .values(ConfigItem {
                    key: "seed".into(),
                    value,
                })
                .execute(&pool.get().expect("Can't connect to db"))
                .expect("Setup seed failed");

            ()
        }
        _ => info!("Seed already present, skipping"),
    }
}

fn migrate_2020_05_15_152936() {
    debug!("Migrating 2020_05_15_152936");
    use schema::objects::dsl::objects;
    use schema::properties::dsl;

    let pool = DbContext::default_pool();
    let ctx = DbContext::new(&pool);
    let db_conn = ctx.get_conn();

    let page_size = 8192;
    let mut cur_offset = 0;

    loop {
        db_conn
            .transaction::<(), diesel::result::Error, _>(|| {
                let props: Result<Vec<Property>, diesel::result::Error> = dsl::properties
                    .offset(cur_offset)
                    .limit(page_size)
                    .get_results::<Property>(&db_conn);
                cur_offset += page_size;

                match props {
                    Ok(props) => {
                        println!(
                            "Process: {}, from: {}",
                            props.len(),
                            props.first().unwrap().id
                        );

                        let values = props.iter().map(|p| &p.value);

                        let mut hashes = vec![];

                        for v in values {
                            hashes.push(Object {
                                hash: Uu128::from(ctx.lookup_table.calculate_hash(&v)),
                                value: String::from(v),
                            });
                        }

                        diesel::insert_into(objects)
                            .values(hashes)
                            .on_conflict_do_nothing()
                            .execute(&db_conn)
                            .expect("Insert failed");

                        Ok(())
                    }
                    Err(e) => {
                        println!("Error: {}", e);

                        Err(RollbackTransaction)
                    }
                }
            })
            .unwrap();
    }
}
