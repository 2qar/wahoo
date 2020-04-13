use serenity::prelude::Context;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{CommandResult, CommandError};
use battlefy;

use wahoo;

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
                return Err(CommandError::from("No team in this server, something stupid happened."));
            },
        },
        Err(e) => {
            return Err(CommandError::from(format!("Error grabbing team: {}", e)));
        },
    };
    let bf = match wahoo::battlefy_config(team_id, &mut db) {
        Ok(r) => match r {
            Some(b) => b,
            None => {
                return Err(CommandError::from("No Battlefy config for this server, use <set_team and <set_tournament."));
            }
        },
        Err(e) => {
            return Err(CommandError::from(format!("Error grabbing battlefy config: {}", e)));
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
                Err(CommandError::from("No match found."))
            },
        },
        Err(e) => {
            Err(CommandError::from(format!("Error grabbing match: {}", e)))
        },
    }
}
