use std::marker::Sync;

use serenity::prelude::Context;
use serenity::model::channel::Message;
use serenity::framework::standard::{Args, Delimiter, CommandResult, CommandError, macros::command};
use regex::Regex;
use postgres::types::ToSql;

use wahoo;
use wahoo::PostgresClient;

fn update_field<T: ToSql + Sync>(field: &str, value: T, ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut data = ctx.data.write();
    let mut db = data.get_mut::<PostgresClient>().expect("error grabbing psql client");
    
    let guild_id = msg.guild_id.unwrap().to_string().parse::<i64>().unwrap();
    let channel_id = msg.channel_id.to_string();
    let team_id = match wahoo::team_id_in(guild_id, &channel_id, &mut db) {
        Ok(r) => match r {
            Some(i) => Ok(i),
            None => Err(CommandError::from("No team in this server, something stupid happened.")),
        },
        Err(e) => Err(CommandError::from(format!("Error grabbing team id: {}", e))),
    }?;

    let query = format!(
        "INSERT INTO battlefy (team, {}) VALUES ($1, $2)
         ON CONFLICT (team) DO UPDATE set {} = $2",
         field, field
    );

    match db.execute(query.as_str(), &[&team_id, &value]) {
        Ok(_) => Ok(()),
        Err(e) => {
            Err(CommandError::from(format!("Error updating database: {}", e)))
        },
    }
}

#[command]
#[num_args(1)]
#[usage("<set_team [team_url]")]
#[description("Update this server's Battlefy team.")]
fn set_team(mut ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut args = Args::new(&msg.content, &[Delimiter::Single(' ')]);
    let arg = args.advance().single::<String>().unwrap();
    let bf_team_id = match find_team_id(&arg) {
        Some(i) => Ok(i),
        None => Err(CommandError::from("Invalid URL.")),
    }?;

    match update_field("team_id", bf_team_id, &mut ctx, &msg) {
        Ok(_) => {
            if let Err(e) = msg.channel_id.say(&ctx.http, "Updated team URL.") {
                eprintln!("error sending status message: {}", e);
            }
            Ok(())
        },
        Err(e) => Err(e),
    }
}

#[command]
#[num_args(1)]
#[usage("<set_tournament [tournament_url]")]
#[description("Update this server's Battlefy tournament.
              The tournament URL has to be from the bracket page of the tournament.")]
fn set_tournament(mut ctx: &mut Context, msg: &Message) -> CommandResult {
    let mut args = Args::new(&msg.content, &[Delimiter::Single(' ')]);
    let arg = args.advance().single::<String>().unwrap();
    let stage_id = match find_stage_id(&arg) {
        Some(i) => Ok(i),
        None => Err(CommandError::from("Invalid URL.")),
    }?;

    match update_field("stage_id", stage_id, &mut ctx, &msg) {
        Ok(_) => {
            if let Err(e) = msg.channel_id.say(&ctx.http, "Updated tournament.") {
                eprintln!("error sending status message: {}", e);
            }
            Ok(())
        },
        Err(e) => Err(e)
    }
}

fn last_url_element<'a>(url: &'a str, re_str: &'a str) -> Option<&'a str> {
    let re = Regex::new(re_str).unwrap();
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

fn find_team_id<'a>(url: &'a str) -> Option<&'a str> {
    last_url_element(url, "https://battlefy.com/teams/.{24}")
}

fn find_stage_id<'a>(url: &'a str) -> Option<&'a str> {
    last_url_element(url, "https://battlefy.com/.+/.+/.{24}/stage/.{24}")
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

    #[test]
    fn find_stage_id_match() {
        let url = "https://battlefy.com/overwatch-open-division-north-america/2019-overwatch-open-division-practice-season-north-america/5d6fdb02c747ff732da36eb4/stage/5d7b716bb7758c268b771f83/bracket/1";
        match find_stage_id(url) {
            Some(s) => assert_eq!(s, "5d7b716bb7758c268b771f83"),
            None => panic!("no stage id found"),
        }
    }

    #[test]
    fn find_stage_id_no_match() {
        let url = "https://battlefy.com/org/tournament/5d6fdb02c747ff732da36eb4";
        match find_stage_id(url) {
            Some(s) => panic!("stage id matched: {}", s),
            None => (),
        }
    }
}
