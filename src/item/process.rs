use log::debug;
use regex::Regex;
use reqwest::blocking::Client;
use serde_json::Value;
use std::collections::HashMap;

use crate::item::Item;

use self::wikipedia::Wikipedia;

mod item_label;
mod page_views;
mod wikipedia;

fn first_letter_to_uppper_case(s1: String) -> String {
    let mut c = s1.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn get_id(item_json: &Value) -> Option<String> {
    let id = item_json["id"].as_str()?;
    return Some(id.to_string());
}

fn get_wikipedia_title(item_json: &Value) -> Option<String> {
    let wikipedia_title = item_json["sitelinks"]["enwiki"].as_str()?;
    return Some(wikipedia_title.to_string());
}

fn get_label(item_json: &Value) -> Option<String> {
    let label = item_json["labels"].get("en")?;
    return Some(label.as_str().unwrap().to_string());
}

fn ok_label(label: &str) -> bool {
    let label_blocklist = [
        // Dates
        r"century",
        r"\d\d\d\d",
        // Meta
        r"wikipedia",
        r"list of",
        // Uninteresting
        r"airport",
        r"flag of",
    ];

    for re in label_blocklist.iter() {
        if Regex::new(re).unwrap().is_match(&label.to_lowercase()) {
            debug!("Is in label blocklist");
            return false;
        }
    }

    return true;
}

fn get_description(item_json: &Value) -> Option<String> {
    let description = item_json["descriptions"].get("en")?;
    let description = first_letter_to_uppper_case(description.as_str().unwrap().to_string());
    return Some(description);
}

fn ok_description(description: &str) -> bool {
    let description_blocklist = [
        // Space objects
        r"galaxy",
        r"constellation",
        r"star",
        r"planet",
        r"nebula",
        r"moon",
        r"supernova",
        r"asteroid",
        r"cluster",
        r"natural satellite",
        // Chemicals
        r"compound",
        r"element",
        // Locations
        r"region",
        r"state",
        // r"country",
        r"capital",
        r"borough",
        r"community",
        r"department",
        r"province",
        r"county",
        r"city",
        r"town",
        r"commune",
        r"federal subject",
        // Niches
        r"football",
        r"basketball",
        r"baseball",
        r"esportiva",
        r"sport",
        r"team",
        // Datetimes
        r"decade",
        r"domain",
        // Animals
        r"species",
    ];

    for re in description_blocklist.iter() {
        if Regex::new(re)
            .unwrap()
            .is_match(&description.to_lowercase())
        {
            debug!("Is in description blocklist");
            return false;
        }
    }

    return true;
}

struct DatePropIdAndYear {
    date_prop_id: String,
    year: i64,
}

fn get_date_prop_id_and_year(
    item_json: &Value,
    date_props: &HashMap<&str, &str>,
) -> Option<DatePropIdAndYear> {
    let mut date: Option<String> = None;
    let mut date_prop_id: Option<String> = None;

    for (prop_id, _) in date_props.into_iter() {
        let date_prop: Option<String> = match item_json["claims"][prop_id].as_array() {
            Some(dates) => {
                if dates.len() == 0 {
                    None
                } else {
                    Some(dates[0].as_str().unwrap().to_string())
                }
            }
            _ => None,
        };

        if let Some(date_prop) = date_prop {
            date = Some(date_prop);
            date_prop_id = Some(prop_id.to_string());
            break;
        }
    }

    if date.is_none() || date_prop_id.is_none() {
        debug!("No date prop found");
        return None;
    }

    let mut date = date.unwrap();
    let date_prop_id = date_prop_id.unwrap();
    let first_char = date.chars().nth(0).unwrap();
    let bce = first_char == '-';

    if bce {
        date = date[1..].to_string();
    }

    let date = date;
    let mut year = date.split("-").collect::<Vec<&str>>()[0]
        .parse::<i64>()
        .unwrap();

    if bce {
        year = year * -1
    }

    let year = year;

    return Some(DatePropIdAndYear { date_prop_id, year });
}

fn get_instance_of(
    item_json: &Value,
    id_label_map: &mut HashMap<String, String>,
    client: &Client,
) -> Option<Vec<String>> {
    return match item_json["claims"]["P31"].as_array() {
        Some(ids) => Some(
            ids.into_iter()
                .map(|id| return item_label::get(id.as_str().unwrap(), id_label_map, &client))
                .filter(|label_option| return label_option.is_some())
                .map(|label_option| return label_option.unwrap())
                .collect(),
        ),
        _ => None,
    };
}

fn ok_instance_of(instance_of: &Vec<String>) -> bool {
    if instance_of.clone().contains(&String::from("taxon")) {
        debug!("Ignore taxon instances");
        return false;
    }

    return true;
}

fn get_occupations(
    item_json: &Value,
    id_label_map: &mut HashMap<String, String>,
    client: &Client,
) -> Option<Vec<String>> {
    return match item_json["claims"]["P106"].as_array() {
        Some(ids) => Some(
            ids.into_iter()
                .map(|id| return item_label::get(id.as_str().unwrap(), id_label_map, &client))
                .filter(|label_option| return label_option.is_some())
                .map(|label_option| return label_option.unwrap())
                .collect(),
        ),
        _ => None,
    };
}

fn get_num_sitelinks(item_json: &Value) -> Option<usize> {
    let num_sitelinks = item_json["sitelinks"].as_object()?;
    return Some(num_sitelinks.keys().len());
}

fn enough_sitelinks(num_sitelinks: usize) -> bool {
    if num_sitelinks < 15 {
        debug!("Not enough sitelinks");
        return false;
    }

    return true;
}

fn enough_page_views(year: i64, instance_of: &Vec<String>, page_views: usize) -> bool {
    if instance_of.contains(&String::from("human")) {
        if year > 1920 && page_views < 100000 {
            return false;
        } else if year > 1900 && page_views < 25000 {
            return false;
        } else if year > 1800 && page_views < 15000 {
            return false;
        } else if page_views < 10000 {
            return false;
        }
    }

    if year > 1960 && page_views < 40000 {
        return false;
    } else if year > 1900 && page_views < 25000 {
        return false;
    } else if year > 1800 && page_views < 15000 {
        return false;
    } else if page_views < 10000 {
        return false;
    }

    return true;
}

pub fn process_item_json(
    item_json: &str,
    date_props: &HashMap<&str, &str>,
    id_label_map: &mut HashMap<String, String>,
    client: &Client,
) -> Option<Item> {
    let item_json: Value = serde_json::from_str(&item_json).unwrap();

    let label = get_label(&item_json)?;

    if !ok_label(&label) {
        return None;
    }

    let description = get_description(&item_json)?;

    if !ok_description(&description) {
        return None;
    }

    let id = get_id(&item_json)?;
    let wikipedia_title = get_wikipedia_title(&item_json)?;
    let DatePropIdAndYear { date_prop_id, year } =
        get_date_prop_id_and_year(&item_json, date_props)?;
    let instance_of = get_instance_of(&item_json, id_label_map, client)?;

    if !ok_instance_of(&instance_of) {
        return None;
    }

    let occupations = get_occupations(&item_json, id_label_map, client);
    let num_sitelinks = get_num_sitelinks(&item_json)?;

    if !enough_sitelinks(num_sitelinks) {
        return None;
    }

    let page_views = page_views::get(&wikipedia_title, client)?;

    if !enough_page_views(year, &instance_of, page_views) {
        return None;
    }

    let Wikipedia { image, label } = wikipedia::get(&wikipedia_title, client)?;

    Some(Item {
        date_prop_id,
        description,
        id,
        image,
        instance_of,
        label,
        occupations,
        page_views,
        wikipedia_title,
        year,
    })
}
