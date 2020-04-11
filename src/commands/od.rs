use serenity::prelude::Context;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{CommandResult, CommandError};

use wahoo::{self, battlefy};

#[command]
pub fn od(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write();
    let mut db = data.get_mut::<wahoo::PostgresClient>().expect("error grabbing psql client");
    let guild_id = msg.guild_id.unwrap().to_string().parse::<i64>().unwrap();
    let channel_id = msg.channel_id.to_string();
    let team_id = match wahoo::team_id_in(guild_id, &channel_id, &mut db) {
        Ok(r) => match r {
            Some(i) => i,
            None => {
                msg.channel_id.say(&ctx.http, "No team in this server / channel.");
                return Err(CommandError("no team in this channel / server".to_string()));
            },
        },
        Err(e) => {
            println!("error grabbing team [guild_id: {}, channel_id: {}]: {}",
                     &msg.guild_id.unwrap(), &msg.channel_id, e);
            msg.channel_id.say(&ctx.http, format!("Error grabbing team: {}", e));
            return Err(CommandError("error grabbing team".to_string()));
        },
    };
    let bf = match wahoo::battlefy_config(team_id, &mut db) {
        Ok(r) => match r {
            Some(b) => b,
            None => {
                msg.channel_id.say(&ctx.http, "No battlefy config for the team in this server / channel.");
                return Err(CommandError("no battlefy config".to_string()));
            }
        },
        Err(e) => {
            println!("error grabbing battlefy config [team_id: {}]: {}", team_id, e);
            msg.channel_id.say(&ctx.http, format!("Error grabbing battlefy config: {}", e));
            return Err(CommandError("error grabbing battlefy config".to_string()));
        }
    };

    match battlefy::matchup(&bf.stage_id, &bf.team_id, 1) {
        Ok(s) => match s {
            Some(t) => {
                msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|mut e| {
                        wahoo::team_embed(t, &mut e);
                        e
                    });
                    m
                });
                Ok(())
            },
            None => {
                msg.channel_id.say(&ctx.http, "No match found.");
                Err(CommandError("no match found".to_string()))
            },
        },
        Err(e) => {
            println!("error grabbing match: {}", e);
            Err(CommandError("error grabbing match".to_string()))
        },
    }
}
