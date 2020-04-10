// TODO: move this file into it's own crate / github repo / whatever it's called
use std::default::Default;
use std::error::Error;
use std::fs::File;
use std::io::BufReader;

use reqwest;
use serde::{self, Deserialize};

const CLOUDFRONT: &'static str = "https://dtmwra1jsgyb0.cloudfront.net/";

pub enum SearchResult {
    None,
    Team(Team),
    Teams(Vec<Team>),
}

#[derive(Deserialize, Default)]
struct Battlenet {
    battletag: String,
}

#[derive(Deserialize, Default)]
struct Accounts {
    battlenet: Battlenet,
}

#[derive(Deserialize, Default)]
struct User {
    name: String,
    accounts: Accounts,
}

#[derive(Deserialize, Default)]
pub struct Player {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "persistentPlayerID")]
    pid: String,
    #[serde(rename = "inGameName")]
    ign: String,
    #[serde(default)]
    user: User,
}

impl Player {
    pub fn battletag(&self) -> Option<&str> {
        if self.ign.len() > 0 {
            Some(&self.ign)
        } else if self.user.accounts.battlenet.battletag.len() > 0 {
            Some(&self.user.accounts.battlenet.battletag)
        } else {
            None
        }
    }
}

#[derive(Deserialize, Default)]
struct PersistentTeam {
    name: String,
    #[serde(rename = "logoUrl")]
    logo_url: String,
    #[serde(rename = "persistentPlayerIDs")]
    persistent_player_ids: Vec<String>,
    #[serde(rename = "persistentCaptainID")]
    persistent_captain_id: String,
}

#[derive(Deserialize, Default)]
pub struct Team {
    #[serde(default)]
    pub players: Vec<Player>,
    #[serde(rename = "persistentTeamID")]
    persistent_team_id: String,
    #[serde(rename = "persistentTeam", default)]
    persistent_team: PersistentTeam,
    #[serde(rename = "_id")]
    id: String,
}

impl Team {
    pub fn find(tournament_id: &str, name: &str) -> Result<SearchResult, reqwest::Error> {
        let url = format!("{}tournaments/{}/teams?name={}", CLOUDFRONT, tournament_id, name);
        let mut teams: Vec<Team> = reqwest::blocking::get(&url)?.json()?;
        
        if teams.len() == 0 {
            Ok(SearchResult::None)
        } else if teams.len() == 1 {
            Ok(SearchResult::Team(teams.pop().unwrap()))
        } else {
            Ok(SearchResult::Teams(teams))
        }
    }

    pub fn name(&self) -> &str {
        &self.persistent_team.name
    }

    pub fn pid(&self) -> &str {
        &self.persistent_team_id
    }

    pub fn logo_url(&self) -> &str {
        &self.persistent_team.logo_url
    }
}

#[derive(Deserialize, Default)]
struct MatchTeam {
    #[serde(default)]
    team: Team,
}

#[derive(Deserialize, Default)]
pub struct Match {
    #[serde(rename = "_id")]
    id: String,
    top: MatchTeam,
    bottom: MatchTeam,
    #[serde(skip)]
    is_top: bool,
}

fn find_match(matches: &mut Vec<Match>, team_id: &str) -> Option<Match> {
    if team_id == "" {
        return None;
    }

    for (i, m) in matches.iter_mut().enumerate() {
        if m.has_team(team_id) {
            return Some(matches.remove(i));
        }
    }

    None
}

impl Match {
    pub fn find(stage_id: &str, team_id: &str, round: u8) -> Result<Option<Match>, reqwest::Error> {
        let url = format!("{}stages/{}/rounds/{}/matches", CLOUDFRONT, stage_id, round);
        let mut matches: Vec<Match> = reqwest::blocking::get(&url)?.json()?;
        
        Ok(find_match(&mut matches, team_id))
    }

    pub fn other_team(&self) -> reqwest::Result<Option<Team>> {
        let pos = if self.is_top {
            "bottom"
        } else {
            "top"
        };
        let url = format!(
            "{}matches/{}?extend[{}.team][players][users]&extend[{}.team][persistentTeam]",
            CLOUDFRONT, self.id, pos, pos
        );
        let mut matches: Vec<Match> = reqwest::blocking::get(&url)?.json()?;

        if matches.len() != 1 {
            return Ok(None);
        }

        if self.is_top {
            Ok(Some(matches.pop().unwrap().bottom.team))
        } else {
            Ok(Some(matches.pop().unwrap().top.team))
        }
    }

    fn has_team(&mut self, team_id: &str) -> bool {
        if self.top.team.persistent_team_id == team_id {
            self.is_top = true;
            true
        } else if self.bottom.team.persistent_team_id == team_id {
            self.is_top = false;
            true
        } else {
            false
        }
    }
}

pub fn matchup(stage_id: &str, team_id: &str, round: u8) -> reqwest::Result<Option<Team>> {
    let m = match Match::find(&stage_id, &team_id, round)? {
        Some(r) => r,
        None => return Ok(None),
    };

    match m.other_team()? {
        Some(t) => Ok(Some(t)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_matches() -> Result<Vec<Match>, Box<dyn Error>> {
        Ok(serde_json::from_reader::<BufReader<File>, Vec<Match>>(
            BufReader::new(File::open("./tests/matches.json")?)
        )?)
    }

    #[test]
    fn find_match_success() -> Result<(), Box <dyn Error>> {
        let mut matches = load_matches()?;

        let team_id = "5bfe1b9418ddd9114f14efb0";
        let match_id = String::from("5e15cb20fba5985c0a8b6b7a");
        match find_match(&mut matches, team_id) {
            Some(m) => assert_eq!(m.id, match_id),
            None => panic!("error finding match"),
        }

        Ok(())
    }

    #[test]
    fn find_match_no_id() -> Result<(), Box <dyn Error>> {
        let mut matches = load_matches()?;

        match find_match(&mut matches, "") {
            Some(m) => panic!("found match with id {}", m.id),
            None => Ok(()),
        }
    }

    #[test]
    fn find_match_no_result() -> Result<(), Box <dyn Error>> {
        let mut matches = load_matches()?;

        match find_match(&mut matches, "test") {
            Some(m) => panic!("found match with id {}", m.id),
            None => Ok(()),
        }
    }

    #[test]
    fn match_has_team() {
        let mut m: Match = Default::default();
        m.top.team.persistent_team_id = String::from("1");
        m.bottom.team.persistent_team_id = String::from("2");

        assert!(m.has_team("1"));
        assert!(m.is_top);
    }

    #[test]
    fn parse_extended_match() -> Result<(), Box<dyn Error>> {
        let _ = serde_json::from_reader::<BufReader<File>, Vec<Match>>(
            BufReader::new(File::open("./tests/match.json")?)
        )?;
        Ok(())
    }

    #[test]
    fn battletag() {
        let mut player: Player = Default::default();
        player.ign = String::from("test#1234");
        assert_eq!(player.battletag(), Some("test#1234"));
    }

    #[test]
    fn battletag_bnet() {
        let mut player: Player = Default::default();
        player.user.accounts.battlenet.battletag = String::from("test#1234");
        assert_eq!(player.battletag(), Some("test#1234"));
    }

    #[test]
    fn battletag_missing() {
        let player: Player = Default::default();
        assert_eq!(player.battletag(), None);
    }
}
