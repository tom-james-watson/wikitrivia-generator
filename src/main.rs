use regex::Regex;
use reqwest::blocking::Client;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::{thread, time};

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

struct Item {
    id: String,
    label: String,
    description: String,
    image: Option<String>,
    date: String,
    date_prop_id: String,
    wikipedia: String,
    num_sitelinks: usize,
    content_length: usize,
    instance_of: Option<Vec<String>>,
}

fn first_letter_to_uppper_case(s1: String) -> String {
    let mut c = s1.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

fn get_content_length(wikipedia: &str, client: &Client) -> Option<usize> {
    // Courtesy light rate limiting
    thread::sleep(time::Duration::from_millis(100));

    let res = client
        .head(format!(
            "https://en.wikipedia.org/wiki/{}",
            wikipedia.replace(" ", "_")
        ))
        .send();

    if res.is_err() {
        dbg!(&res);
        return None;
    }

    let res = res.unwrap();
    let content_length = &res.headers().get("content-length");

    if content_length.is_none() {
        println!("No content-length header");
        return None;
    }

    let content_length = content_length.unwrap().to_str().unwrap().parse::<usize>();

    if content_length.is_err() {
        dbg!(&content_length);
        return None;
    }

    Some(content_length.unwrap())
}

fn process_item_json(
    item_json: &str,
    date_props: &HashMap<&str, &str>,
    client: &Client,
) -> Option<Item> {
    // println!("\n---------------------------------------------\n");

    let v: serde_json::Value = serde_json::from_str(&item_json).unwrap();

    let id = v["id"].as_str().unwrap().to_string();

    let label = v["labels"].get("en");

    if label.is_none() {
        // println!("No description");
        return None;
    }

    let label = label.unwrap().as_str().unwrap().to_string();

    // println!("{}", &label);

    let wikipedia = v["sitelinks"]["enwiki"].as_str();

    if wikipedia.is_none() {
        // println!("No wikipedia link");
        return None;
    }

    let wikipedia = wikipedia.unwrap().to_string();

    let description = v["descriptions"].get("en");

    if description.is_none() {
        // println!("No description");
        return None;
    }

    let description =
        first_letter_to_uppper_case(description.unwrap().as_str().unwrap().to_string());

    let description_blocklist_res = [
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
        r"country",
        r"capital",
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

    // TODO:
    // * Have different minimum sitelinks and content lengths per type of item. Higher for people,
    // for example.
    // * Also have different minimums as a function of age. The older something is, the less
    // sitelinks and content it should need.

    for re in description_blocklist_res.iter() {
        if Regex::new(re)
            .unwrap()
            .is_match(&description.to_lowercase())
        {
            // println!("Is in description blocklist");
            return None;
        }
    }

    let label_blocklist_res = [
        // Dates
        r"century",
        r"\d\d\d\d",
        // Meta
        r"wikipedia",
        r"list of",
        // Uninteresting
        r"airport",
    ];

    for re in label_blocklist_res.iter() {
        if Regex::new(re).unwrap().is_match(&label.to_lowercase()) {
            // println!("Is in label blocklist");
            return None;
        }
    }

    // Note: can get an image with special redirect file:
    // https://commons.wikimedia.org/w/index.php?title=Special:Redirect/file/Sample.png&width=300
    let image: Option<String> = match v["claims"]["P18"].as_array() {
        Some(image_urls) => {
            if image_urls.len() == 0 {
                None
            } else {
                Some(image_urls[0].as_str().unwrap().to_string())
            }
        }
        _ => None,
    };

    let mut date: Option<String> = None;
    let mut date_prop_id: Option<String> = None;

    for (prop_id, _) in date_props.into_iter() {
        let date_prop: Option<String> = match v["claims"][prop_id].as_array() {
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
        // println!("No date prop found");
        return None;
    }

    let date = date.unwrap();
    let date_prop_id = date_prop_id.unwrap();

    // TODO - remove once we reprocess processed.json
    if date_prop_id == "P569" {
        // println!("Ignore birth date");
        return None;
    }

    let instance_of: Option<Vec<String>> = match v["claims"]["P31"].as_array() {
        Some(ids) => Some(
            ids.into_iter()
                .map(|id| id.as_str().unwrap().to_string())
                .collect(),
        ),
        _ => None,
    };

    let num_sitelinks = v["sitelinks"].as_object().unwrap().keys().len();

    if num_sitelinks < 20 {
        // println!("Not enough sitelinks");
        return None;
    }

    let content_length = get_content_length(&wikipedia, client);

    if content_length.is_none() {
        println!(
            "Can't fetch content length (https://en.wikipedia.org/wiki/{})",
            wikipedia.replace(" ", "_")
        );
        return None;
    }

    let content_length = content_length.unwrap();

    if content_length < 100000 {
        println!(
            "Wikipedia article too short (https://en.wikipedia.org/wiki/{})",
            wikipedia.replace(" ", "_")
        );
        return None;
    }

    // Reject reasons
    // date is older than xxx years (100,000?). Avoids things like species

    Some(Item {
        id,
        label,
        description,
        image,
        date,
        date_prop_id,
        wikipedia,
        num_sitelinks,
        instance_of,
        content_length,
    })
}

fn main() {
    let mut count: isize = 0;
    let mut seen_count: isize = 0;

    // The order of this matters - they should be ranked in order of importance. The prop that is
    // found first is the prop that will be used for the item.
    let date_props: HashMap<&str, &str> = [
        ("P575", "time of discovery or invention"),
        ("P7589", "date of assent"),
        ("P577", "publication date"),
        ("P1191", "date of first performance"),
        ("P1619", "date of official opening"),
        ("P571", "inception"),
        ("P1249", "time of earliest written record"),
        ("P576", "dissolved, abolished or demolished date"),
        ("P8556", "extinction date"),
        ("P6949", "announcement date"),
        ("P1319", "earliest date"),
        ("P570", "date of death"),
        ("P582", "end time"),
        ("P580", "start time"),
        ("P7124", "date of the latest one"),
        ("P7124", "date of the first one"),
    ]
    .iter()
    .cloned()
    .collect();

    // File hosts must exist in current path before this produces output
    let lines = read_lines("./processed.json").unwrap();

    let client = Client::builder().build().unwrap();

    for line in lines {
        seen_count += 1;
        if let Ok(item_json) = line {
            if let Some(item) = process_item_json(&item_json, &date_props, &client) {
                count += 1;
                println!("\n---------------------------------------------\n");
                println!("Count={} Seen={}", count, seen_count);
                println!("{}", &item.id);
                println!("{}", &item.label);
                println!("{}", &item.description);
                if let Some(img) = item.image {
                    println!("https://commons.wikimedia.org/w/index.php?title=Special:Redirect/file/{}&width=300", urlencoding::encode(&img));
                }
                println!(
                    "{}: {}",
                    &date_props.get(&*item.date_prop_id).unwrap(),
                    &item.date
                );
                println!(
                    "https://en.wikipedia.org/wiki/{}",
                    item.wikipedia.replace(" ", "_")
                );
                println!("num_sitelinks: {}", &item.num_sitelinks);
                println!("content_length: {}", &item.content_length);
                if let Some(instance_of) = item.instance_of {
                    dbg!(&instance_of);
                }
            }
        }
    }

    println!("Total: Count={} Seen={}", count, seen_count);
}
