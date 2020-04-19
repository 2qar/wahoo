use serenity::prelude::Context;
use serenity::model::channel::Message;
use serenity::framework::standard::macros::command;
use serenity::framework::standard::{Args, Delimiter, CommandResult, CommandError};
use battlefy;

use wahoo;

#[command]
#[num_args(1)]
#[usage("<round [round_num]")]
#[description("Grab stats on the team you're matched with in a round.")]
pub fn round(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut args = Args::new(&msg.content, &[Delimiter::Single(' ')]);
    let arg = args.advance().single_quoted::<String>().unwrap();

    let bf = battlefy_config(ctx, msg)?;
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
        Err(_) => Err(CommandError::from(format!("Expected a number, got \"{}\"", arg))),
    }?;

    if let Err(e) = msg.channel_id.send_message(&ctx.http, |m| {
        m.embed(|mut e| {
            wahoo::team_embed(team, &mut e);
            e
        })
    }) {
        eprintln!("error sending message: {}", e);
    }
    Ok(())
}

#[command]
#[num_args(1)]
#[usage("<team \"[name]\"")]
#[description("Search for a team in this tournament, and show their stats.")]
fn team(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut args = Args::new(&msg.content, &[Delimiter::Single(' ')]);
    let name = args.advance().single_quoted::<String>().unwrap();

    let bf = battlefy_config(ctx, msg)?;
    // FIXME: stupid, just add a tournament_id field to the battlefy table
    let tournament_id = &bf.tournament_link[111..135];
    match battlefy::Team::find(tournament_id, &name) {
        Ok(r) => match r {
            battlefy::SearchResult::Team(t) => {
                if let Err(e) = msg.channel_id.send_message(&ctx.http, |m| {
                    m.embed(|mut e| {
                        wahoo::team_embed(t, &mut e);
                        e
                    })
                }) {
                    eprintln!("error sending message: {}", e);
                }
                Ok(())
            },
            battlefy::SearchResult::Teams(teams) => {
                let mut message = String::from("```\n");
                for team in teams {
                    message.push_str(format!("{}\n", team.name()).as_str());
                }
                message.push_str("```");
                if let Err(e) = msg.channel_id.say(&ctx.http, message) {
                    eprintln!("error sending team search list: {}", e);
                }

                Ok(())
            },
            battlefy::SearchResult::None => Err(CommandError::from("No teams found.")),
        },
        Err(e) => Err(CommandError::from(format!("Error searching: {}", e))),
    }

}

fn battlefy_config(ctx: &Context, msg: &Message) -> Result<wahoo::BattlefyConfig, CommandError> {
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
    match wahoo::battlefy_config(team_id, &mut db) {
        Ok(r) => match r {
            Some(b) => Ok(b),
            None => Err(CommandError::from("No Battlefy config for this server, use <set_team and <set_tournament.")),
        },
        Err(e) => Err(CommandError::from(format!("Error grabbing battlefy config: {}", e))),
    }
}
