use log::warn;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::{thread, time};

#[derive(Serialize, Deserialize)]
struct Label {
    language: String,
    value: String,
}

#[derive(Serialize, Deserialize)]
struct EntityData {
    #[serde(rename = "type")]
    entity_type: String,
    id: String,
    labels: HashMap<String, Label>,
}

#[derive(Serialize, Deserialize)]
struct EntitiesResponse {
    entities: HashMap<String, EntityData>,
}

pub fn get(
    id: &str,
    id_label_map: &mut HashMap<String, String>,
    client: &Client,
) -> Option<String> {
    if let Some(label) = id_label_map.get(id) {
        return Some(label.to_string());
    }

    let res = client
        .get(format!(
            "https://www.wikidata.org/w/api.php?action=wbgetentities&props=labels&ids={}&languages=en&format=json",
            id
        ))
        .header(USER_AGENT, "wiki-game analysis by wiki-game@tomjwatson.com")
        .send();

    if let Err(err) = res {
        warn!("Got an error from wikidata API: {}", err);
        return None;
    }

    let res = res.unwrap();

    if res.status() == 429 {
        warn!("Rate limited by wikidata API, waiting 30 seconds");
        thread::sleep(time::Duration::from_secs(30));
    }

    let json = res.json::<EntitiesResponse>();

    if let Err(err) = json {
        warn!("Can't parse wikidata response: {}", err);
        return None;
    }

    let json = json.unwrap();
    let label = json.entities.get(id)?.labels.get("en")?.value.to_string();

    id_label_map.insert(id.to_string(), label.to_string());

    return Some(label);
}
