use log::warn;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::{thread, time};

#[derive(Serialize, Deserialize)]
pub struct Items {
    pub access: String,
    pub agent: String,
    pub article: String,
    pub granularity: String,
    pub project: String,
    pub timestamp: String,
    pub views: usize,
}

#[derive(Serialize, Deserialize)]
struct PageViewsResponse {
    items: Vec<Items>,
}

pub fn get(wikipedia: &str, client: &Client) -> Option<usize> {
    let res = client
        .get(format!(
            "https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/en.wikipedia/all-access/all-agents/{}/monthly/2021010100/2021020100",
            wikipedia.replace(" ", "_")
        ))
        .header(USER_AGENT, "wiki-game analysis by wiki-game@tomjwatson.com")
        .send();

    if let Err(err) = res {
        warn!("Got an error from page views API: {}", err);
        return None;
    }

    let res = res.unwrap();

    if res.status() == 429 {
        warn!("Rate limited by page views API, waiting 30 seconds");
        thread::sleep(time::Duration::from_secs(30));
    }

    let json = res.json::<PageViewsResponse>();

    if let Err(err) = json {
        warn!("Can't parse page views response: {}", err);
        return None;
    }

    let json = json.unwrap();

    if json.items.len() == 0 {
        warn!("Empty items returned from page views API");
        return None;
    }

    return Some(json.items[0].views);
}
