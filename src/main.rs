use std::error::Error;
use std::env;

use serenity;
use serenity::model::guild::Guild;
use serenity::prelude::{EventHandler, Context};
use serenity::framework::standard::macros::group;
use serenity::framework::standard::StandardFramework;
use postgres::{self, NoTls};

mod commands;
// FIXME: this seems kinda hacky
use crate::commands::od;
use crate::commands::od::OD_COMMAND;
use crate::commands::config;
use crate::commands::config::SET_TEAM_COMMAND;
use crate::commands::config::SET_TOURNAMENT_COMMAND;

use wahoo::PostgresClient;

#[group]
#[commands(od, set_team, set_tournament)]
struct General;

struct Handler;
impl EventHandler for Handler {
    fn guild_create(&self, ctx: Context, guild: Guild, is_new: bool) {
        if is_new {
            let mut data = ctx.data.write();
            let mut db = data.get_mut::<PostgresClient>().expect("error grabbing psql client");

            let guild_id = guild.id.to_string()
                .parse::<i64>()
                .unwrap();

            match db.query_opt(
                "SELECT server_id, team_name FROM teams WHERE server_id = $1 AND team_name = ''",
                &[&guild_id]
            ) {
                Ok(r) => match r {
                    Some(_) => {
                        return;
                    }
                    None => (),
                }
                Err(e) => {
                    eprintln!("error querying db: {}", e);
                    return;
                }
            }

            match db.execute(
                "INSERT INTO teams (server_id, team_name) VALUES ($1, '')",
                &[&guild_id]
            ) {
                Ok(_) => (),
                Err(e) => {
                    eprintln!("error adding guild team [id {}]: {}", guild.id, e);
                },
            }
        }
    }
}

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
        data.insert::<PostgresClient>(pg_client);
    }

    // TODO: handle command errors with .after()
    //       so i don't have to write a msg.channel_id.say
    //       for every single error and write the error twice
    discord_client.with_framework(StandardFramework::new()
        .configure(|c| c.prefix("<"))
        .group(&GENERAL_GROUP));
    discord_client.start()?;

    Ok(())
}
