// TODO: move this file into it's own crate / github repo / whatever it's called
use select::document::Document;
use select::predicate::{Class, Predicate, Attr, Name};
use select::node::Data;
use reqwest::{self, header::USER_AGENT};

#[derive(Debug)]
pub enum Role {
    None,
    Tank,
    Offense,
    Support,
    Defense,
}

#[derive(Debug)]
pub struct Player {
    pub battletag: String,
    pub sr: u16,
    pub role: Role,
}

impl Player {
    pub fn find(client: &reqwest::blocking::Client, battletag: &str) -> reqwest::Result<Player> {
        let url = format!("https://www.overbuff.com/players/pc/{}", battletag.to_string().replace("#", "-"));
        let html = client.get(&url)
            .header(USER_AGENT, "wahoo :)")
            .send()?.text()?;
        let document = Document::from(html.as_ref());

        Ok(Player {
            battletag: battletag.to_string(),
            sr: parse_sr(&document).unwrap_or(0),
            role: parse_role(&document),
        })
    }
}

fn parse_battletag(document: &Document) -> Option<String> {
    match document.find(Name("h1")).next() {
        Some(n) => {
            let name = match n.first_child() {
                Some(c) => match c.data() {
                    Data::Text(t) => Some(t.to_string()),
                    _ => None,
                },
                None => None,
            };
            if let None = name {
                return None;
            }

            let tag = match n.find(Name("small")).next() {
                Some(n) => match n.first_child() {
                    Some(c) => match c.data() {
                        Data::Text(t) => Some(t.to_string()),
                        _ => None,
                    },
                    None => None,
                }
                None => None,
            };
            if let None = tag {
                return None;
            }

            let mut name = name.unwrap();
            name.push_str(tag.unwrap().as_ref());
            Some(name)
        },
        None => None,
    }
}

// TODO: make an error type w/ the node it failed at at instead of just returning None, maybe
fn parse_sr(document: &Document) -> Option<u16> {
    match document.find(Class("player-skill-rating")).next() {
        Some(n) => match n.first_child() {
            Some(n) => match n.data() {
                Data::Text(s) => match s.to_string().trim().parse::<u16>() {
                    Ok(i) => Some(i),
                    Err(_) => None,
                }
                _ => None,
            },
            None => None,
        },
        None => None,
    }
}

fn parse_role(document: &Document) -> Role {
    match document.find(Attr("data-portable", "roles")).next() {
        Some(n) => match n.find(Class("stripe-rows").child(Name("tr"))).next() {
            Some(n) => match n.find(Class("color-white")).next() {
                Some(r) => match r.first_child() {
                    Some(c) => match c.data() {
                        Data::Text(r) => match r.as_ref() {
                            "Tank" => Role::Tank,
                            "Offense" => Role::Offense,
                            "Support" => Role::Support,
                            "Defense" => Role::Defense,
                            _ => Role::None,
                        },
                        _ => Role::None,
                    },
                    _ => Role::None,
                },
                None => Role::None,
            },
            None => Role::None,
        },
        None => Role::None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_document() -> Document {
        Document::from(include_str!("../tests/heckoffnerd-1772.html"))
    }

    fn missing_info_document() -> Document {
        Document::from(include_str!("../tests/Tucker-1475.html"))
    }

    fn competitive_document() -> Document {
        Document::from(include_str!("../tests/Ichi-21148.html"))
    }

    #[test]
    fn test_parse_battletag() {
        let document = test_document();
        match parse_battletag(&document) {
            Some(s) => assert_eq!(s, "heckoffnerd#1772"),
            None => panic!("error parsing battletag"),
        }
    }

    #[test]
    fn test_parse_sr() {
        let document = test_document();
        match parse_sr(&document) {
            Some(i) => assert_eq!(i, 2770),
            None => panic!("unable to parse sr"),
        }
    }

    #[test]
    fn parse_sr_missing() {
        let document = missing_info_document();
        if let Some(i) = parse_sr(&document) {
            panic!("parsed sr: {}", i);
        }
    }

    #[test]
    fn parse_sr_competitive() {
        let document = competitive_document();
        match parse_sr(&document) {
            Some(i) => assert_eq!(i, 4692),
            None => panic!("unable to parse sr"),
        }
    }

    #[test]
    fn test_parse_role() {
        let document = test_document();
        match parse_role(&document) {
            Role::Tank => (),
            _ => panic!("wrong role"),
        }
    }

    #[test]
    fn parse_role_missing() {
        let document = missing_info_document();
        match parse_role(&document) {
            Role::None => (),
            _ => panic!("parsed a role"),
        }
    }

    #[test]
    fn parse_role_competitive() {
        let document = competitive_document();
        let role = parse_role(&document);
        match role {
            Role::Tank => (),
            Role::None => panic!("no role parsed"),
            _ => panic!("wrong role parsed"),
        }
    }
}
