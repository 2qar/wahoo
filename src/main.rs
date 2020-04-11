use std::error::Error;
use std::env;

use serenity;
use serenity::prelude::EventHandler;
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use postgres::{self, NoTls};

mod commands;
// FIXME: this seems kinda hacky
use crate::commands::od::OD_COMMAND;

#[group]
#[commands(od)]
struct General;

struct Handler;
impl EventHandler for Handler {}

fn main() -> Result<(), Box<dyn Error>> {
    let token = env::var("WAHOO_TOKEN").expect("$WAHOO_TOKEN not set");
    let mut discord_client = serenity::Client::new(&token, Handler)?;

    let host = env::var("WAHOO_PG_HOST").expect("$WAHOO_PG_HOST not set");
    let user = env::var("WAHOO_PG_USER").expect("$WAHOO_PG_USER not set");
    let pass = env::var("WAHOO_PG_PASS").expect("$WAHOO_PG_PASS not set");
    let db = env::var("WAHOO_PG_DB").expect("$WAHOO_PG_DB not set");

    let conn_string = format!(
        "host={} user={} password={} dbname={}",
        host, user, pass, db
    );
    let mut pg_client = postgres::Client::connect(&conn_string, NoTls)?;

    {
        let mut data = discord_client.data.write();
        data.insert::<wahoo::PostgresClient>(pg_client);
    }

    discord_client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("<"))
        .group(&GENERAL_GROUP));
    discord_client.start()?;

    Ok(())
}
