use std::error::Error;
use std::env;

mod battlefy;
mod overbuff;

use serenity::{self, prelude::*, model::channel::Message, builder::CreateEmbed};
use postgres::{self, NoTls};
use reqwest;

struct PostgresClient;
impl TypeMapKey for PostgresClient {
    type Value = postgres::Client;
}

fn team_id_in(guild_id: i64, channel_id: &str, pg: &mut postgres::Client) -> Result<Option<i32>, postgres::error::Error> {
    let mut row = pg.query_opt(
        "SELECT id FROM teams WHERE server_id = $1 AND $2 = ANY(channels)",
        &[&guild_id, &channel_id]
    )?;

    if let None = row {
        row = pg.query_opt(
            "SELECT id FROM teams WHERE server_id = $1 AND team_name = ''",
            &[&guild_id]
        )?;
        if let None = row {
            return Ok(None);
        }
    }

    match row.unwrap().try_get::<&str, i32>("id") {
        Ok(i) => Ok(Some(i)),
        Err(e) => {
            println!("fuck");
            Err(e)
        },
    }
}

struct BattlefyConfig {
    stage_id: String,
    team_id: String,
    tournament_link: String,
}

fn battlefy_config(team_id: i32, pg: &mut postgres::Client) -> Result<Option<BattlefyConfig>, postgres::error::Error> {
    let row = pg.query_opt(
        "SELECT stage_id, team_id, tournament_link FROM battlefy WHERE team = $1",
        &[&team_id]
    )?;

    if let None = row {
        return Ok(None);
    }

    let cfg = row.unwrap();
    Ok(Some(BattlefyConfig {
        stage_id: cfg.get::<&str, String>("stage_id"),
        team_id: cfg.get::<&str, String>("team_id"),
        tournament_link: cfg.get::<&str, String>("tournament_link"),
    }))
}

struct Handler;
impl EventHandler for Handler {
    fn message(&self, ctx: Context, msg: Message) {
        if msg.author.bot {
            return
        }
        
        if msg.content == "<od" {
            let mut data = ctx.data.write();
            let mut db = data.get_mut::<PostgresClient>().expect("error grabbing psql client");
            let team_id = match team_id_in(msg.guild_id.unwrap().to_string().parse::<i64>().unwrap(), &msg.channel_id.to_string(), &mut db) {
                Ok(r) => match r {
                    Some(i) => i,
                    None => {
                        msg.channel_id.say(&ctx.http, "No team in this server / channel.");
                        return;
                    },
                },
                Err(e) => {
                    println!("error grabbing team [guild_id: {}, channel_id: {}]: {}",
                             &msg.guild_id.unwrap(), &msg.channel_id, e);
                    msg.channel_id.say(&ctx.http, format!("Error grabbing team: {}", e));
                    return;
                },
            };
            let bf = match battlefy_config(team_id, &mut db) {
                Ok(r) => match r {
                    Some(b) => b,
                    None => {
                        msg.channel_id.say(&ctx.http, "No battlefy config for the team in this server / channel.");
                        return;
                    }
                },
                Err(e) => {
                    println!("error grabbing battlefy config [team_id: {}]: {}", team_id, e);
                    msg.channel_id.say(&ctx.http, format!("Error grabbing battlefy config: {}", e));
                    return;
                }
            };

            match battlefy::matchup(&bf.stage_id, &bf.team_id, 1) {
                Ok(s) => match s {
                    Some(t) => {
                        msg.channel_id.send_message(&ctx.http, |m| {
                            m.embed(|mut e| {
                                team_embed(t, &mut e);
                                e
                            });
                            m
                        });
                    },
                    None => {
                        msg.channel_id.say(&ctx.http, "No match found.");
                    },
                },
                Err(e) => println!("error grabbing match: {}", e),
            }
        }
    }
}

fn overbuff_players(players: &Vec<battlefy::Player>) -> Vec<overbuff::Player> {
    let players: Vec<&battlefy::Player> = players.iter().filter(|p| p.battletag().is_some()).collect();

    let client = reqwest::blocking::Client::new();
    let mut overbuff_players: Vec<overbuff::Player> = Vec::new();
    // TODO: use threads, maybe
    for player in players {
        match overbuff::Player::find(&client, player.battletag().unwrap()) {
            Ok(p) => overbuff_players.push(p),
            Err(e) => println!("error grabbing player: {}", e),
        };
    }

    overbuff_players
}

fn team_embed(team: battlefy::Team, e: &mut CreateEmbed) {
    let team_url = format!("https://battlefy.com/teams/{}", team.pid());
    e.author(|a| a.name(&team.name()).url(team_url).icon_url("http://s3.amazonaws.com/battlefy-assets/helix/images/logos/logo.png"));
    e.color(0xe74c3c);
    e.footer(|f| f.text("SR is scraped from Overbuff, and may not be accurate."));
    e.thumbnail(team.logo_url());
    
    let mut players = overbuff_players(&team.players);
    players.sort_by(|p1, p2| p2.sr.cmp(&p1.sr));

    let players_str = players.iter().fold(String::from(""), |acc, p| {
        let role = match p.role {
            overbuff::Role::None => ":grey_question:",
            overbuff::Role::Tank => ":shield:",
            overbuff::Role::Offense => ":crossed_swords:",
            overbuff::Role::Support => ":ambulance:",
            overbuff::Role::Defense => ":crossed_swords:",
        };

        let sr = if p.sr > 0 {
            p.sr.to_string()
        } else {
            String::from(":grey_question:")
        };

        format!("{}{} {}: {}\n", acc, role, p.battletag, sr)
    });

    let top = players.iter().take(6);
    let top_len = top.size_hint().0 as u16;
    let top_sr_avg = top.fold(0, |acc, p| acc + p.sr) / top_len;
    e.field(format!("Top 6 Average: {}", top_sr_avg), players_str, false);
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

    discord_client.start()?;

    Ok(())
}
