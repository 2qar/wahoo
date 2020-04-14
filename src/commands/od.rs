use serenity::prelude::Context;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, Delimiter, CommandResult, CommandError};
use battlefy;

use wahoo;

#[command]
#[num_args(1)]
#[usage("<od [round_num OR team_name]")]
#[description("Search for a team by name, or by round number.
              Round number grabs stats on the team matched with you in that round.")]
pub fn od(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut args = Args::new(&msg.content, &[Delimiter::Single(' ')]);
    let arg = match args.advance().single_quoted::<String>() {
        Ok(s) => Ok(s),
        Err(e) => Err(CommandError::from("<od [round OR name]")),
    }?;

    let mut data = ctx.data.write();
    let mut db = data.get_mut::<wahoo::PostgresClient>().expect("error grabbing psql client");
    let guild_id = msg.guild_id.unwrap().to_string().parse::<i64>().unwrap();
    let channel_id = msg.channel_id.to_string();
    let team_id = match wahoo::team_id_in(guild_id, &channel_id, &mut db) {
        Ok(r) => match r {
            Some(i) => Ok(i),
            None => Err(CommandError::from("No team in this server, something stupid happened.")),
        },
        Err(e) => Err(CommandError::from(format!("Error grabbing team: {}", e))),
    }?;
    let bf = match wahoo::battlefy_config(team_id, &mut db) {
        Ok(r) => match r {
            Some(b) => Ok(b),
            None => Err(CommandError::from("No Battlefy config for this server, use <set_team and <set_tournament.")),
        },
        Err(e) => Err(CommandError::from(format!("Error grabbing battlefy config: {}", e))),
    }?;

    let team = match arg.parse::<i32>() {
        Ok(i) => {
            if i < 0 {
                return Err(CommandError::from("that's not how it works"));
            }

            match battlefy::matchup(&bf.stage_id, &bf.team_id, i as u8) {
                Ok(s) => match s {
                    Some(t) => Ok(t),
                    None => Err(CommandError::from("No match found.")),
                },
                Err(e) => Err(CommandError::from(format!("Error grabbing match: {}", e))),
            }
        },
        Err(_) => {
            // FIXME: stupid, just add a tournament_id field to the battlefy table
            let tournament_id = &bf.tournament_link[111..135];

            match battlefy::Team::find(tournament_id, &arg) {
                Ok(r) => match r {
                    battlefy::SearchResult::Team(t) => Ok(t),
                    battlefy::SearchResult::Teams(teams) => {
                        let mut message = String::from("```\n");
                        for team in teams {
                            message.push_str(format!("{}\n", team.name()).as_str());
                        }
                        message.push_str("```");
                        msg.channel_id.say(&ctx.http, message);
                        return Ok(());
                    },
                    battlefy::SearchResult::None => Err(CommandError::from("No teams found.")),
                },
                Err(e) => Err(CommandError::from(format!("Error searching: {}", e))),
            }
        }
    }?;

    msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|mut e| {
            wahoo::team_embed(team, &mut e);
            e
        })
    });
    Ok(())
}
