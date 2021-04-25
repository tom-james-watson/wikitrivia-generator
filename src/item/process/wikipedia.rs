use log::warn;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::{thread, time};

#[derive(Serialize, Deserialize)]
struct Normalized {
    fromencoded: bool,
    from: String,
    to: String,
}

#[derive(Serialize, Deserialize)]
struct Thumbnail {
    source: String,
    width: i32,
    height: i32,
}

#[derive(Serialize, Deserialize)]
struct Page {
    pageid: i32,
    ns: i32,
    title: String,
    thumbnail: Thumbnail,
    pageimage: String,
}

#[derive(Serialize, Deserialize)]
struct Query {
    normalized: Option<Vec<Normalized>>,
    pages: Vec<Page>,
}

#[derive(Serialize, Deserialize)]
struct WikipediaResponse {
    batchcomplete: bool,
    query: Query,
}

pub struct Wikipedia {
    pub label: String,
    pub image: String,
}

pub fn get(wikipedia: &str, client: &Client) -> Option<Wikipedia> {
    let res = client
        .get(format!(
            "https://en.wikipedia.org/w/api.php?action=query&format=json&formatversion=2&prop=pageimages&titles={}",
            wikipedia.replace(" ", "_")
        ))
        .header(USER_AGENT, "wiki-game analysis by wiki-game@tomjwatson.com")
        .send();

    if let Err(err) = res {
        warn!("Got an error from wikipedia API: {}", &err);
        return None;
    }

    let res = res.unwrap();

    if res.status() == 429 {
        warn!("Rate limited by wikipedia API, waiting 30 seconds");
        thread::sleep(time::Duration::from_secs(30));
    }

    let json = res.json::<WikipediaResponse>();

    if let Err(err) = json {
        warn!("Can't parse wikipedia response: {}", err);
        return None;
    }

    let json = json.unwrap();

    if json.query.pages.len() == 0 {
        warn!("Empty pages returned from page views API");
        return None;
    }

    return Some(Wikipedia {
        label: json.query.pages[0].title.to_string(),
        image: json.query.pages[0].pageimage.to_string(),
    });
}
