use serenity::builder::CreateEmbed;
use serenity::prelude::TypeMapKey;
use serenity::framework::standard::CommandError;
use postgres;
use reqwest;
use battlefy;
use overbuff;

pub struct PostgresClient;
impl TypeMapKey for PostgresClient {
    type Value = postgres::Client;
}

pub struct BattlefyConfig {
    pub stage_id: String,
    pub team_id: String,
    pub tournament_link: String,
}

pub fn battlefy_config(team_id: i32, pg: &mut postgres::Client) -> Result<Option<BattlefyConfig>, postgres::error::Error> {
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

pub fn team_id_in(guild_id: i64, channel_id: &str, pg: &mut postgres::Client) -> Result<Option<i32>, postgres::error::Error> {
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

pub fn team_embed(team: battlefy::Team, e: &mut CreateEmbed) {
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

// strips the outer CommandError("...") from the inner error message
// hacky, but it works :)
pub fn error_to_string(e: CommandError) -> String {
    let e_str = format!("{:?}", e);
    let len = e_str.len();
    e_str[14..len-2].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_to_string() {
        let e = CommandError::from("Error doing something");
        assert_eq!(error_to_string(e), "Error doing something");
    }
}
