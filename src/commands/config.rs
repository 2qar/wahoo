use serenity::prelude::Context;
use serenity::model::channel::Message;
use serenity::framework::standard::{Args, Delimiter, CommandResult, CommandError, macros::command};
use regex::{Regex, Match};

use wahoo;
use wahoo::PostgresClient;

#[command]
//#[num_args(1)] // TODO: <- use this, but fix the macro scope problem first
fn set_team(ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut args = Args::new(&msg.content, &[Delimiter::Single(' ')]);
    let arg = match args.advance().single::<String>() {
        Ok(s) => s,
        Err(_) => {
            msg.channel_id.say(&ctx.http, "No link given.");
            return Err(CommandError("no link given".to_string()));
        }
    };
    let bf_team_id = match find_team_id(&arg) {
        Some(i) => i,
        None => {
            msg.channel_id.say(&ctx.http, "Invalid URL.");
            return Err(CommandError("invalid url".to_string()));
        },
    };

    let mut data = ctx.data.write();
    let mut db = data.get_mut::<PostgresClient>().expect("error grabbing psql client");
    
    let guild_id = msg.guild_id.unwrap().to_string().parse::<i64>().unwrap();
    let channel_id = msg.channel_id.to_string();
    let team_id = match wahoo::team_id_in(guild_id, &channel_id, &mut db) {
        Ok(r) => match r {
            Some(i) => i,
            None => {
                eprintln!("no team in [guild_id: {}]", guild_id);
                return Err(CommandError("no team in guild".to_string()));
            }
        },
        Err(e) => {
            eprintln!("error grabbing team id: {}", e);
            return Err(CommandError("error grabbing team id".to_string()));
        }
    };

    match db.execute(
        "INSERT INTO battlefy (team, team_id) VALUES ($1, $2)
         ON CONFLICT (team) DO UPDATE set team_id = $2",
        &[&team_id, &bf_team_id]
    ) {
        Ok(_) => {
            msg.channel_id.say(&ctx.http, "Updated team URL.");
            Ok(())
        }
        Err(e) => {
            msg.channel_id.say(&ctx.http,
                format!("Error updating database. Maybe try again, unless this looks really bad: {}",
                    e));
            eprintln!("error updating team link [guild id {}]: {}", guild_id, e);
            Err(CommandError("error updating database".to_string()))
        },
    }
}

#[command]
fn set_tournament(ctx: &mut Context, msg: &Message) -> CommandResult {
    Ok(())
}

fn find_team_id<'a>(url: &'a str) -> Option<&'a str> {
    let re = Regex::new("https://battlefy.com/teams/.{24}").unwrap();
    match re.find(url) {
        Some(s) => {
            let match_str = s.as_str();
            match match_str.rfind("/") {
                Some(i) => Some(&match_str[i+1..]),
                None => None,
            }
        },
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn team_link_match() {
        let url = "https://battlefy.com/teams/5bfe1b9418ddd9114f14efb0";
        match find_team_id(url) {
            Some(m) => assert_eq!(m, "5bfe1b9418ddd9114f14efb0"),
            None => panic!("url not matched!"),
        }
    }

    #[test]
    fn team_link_no_match() {
        let url = "https://battlefy.com/teams/totally-fake-url";
        match find_team_id(url) {
            Some(m) => panic!("matched {}", m),
            None => (),
        }
    }
}
