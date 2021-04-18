use io::Write;
use regex::Regex;
use reqwest::blocking::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize)]
struct Item {
    id: String,
    label: String,
    description: String,
    image: Option<String>,
    date: String,
    date_prop_id: String,
    wikipedia: String,
    num_sitelinks: usize,
    page_views: usize,
    types: Vec<String>,
}

fn first_letter_to_uppper_case(s1: String) -> String {
    let mut c = s1.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().to_string() + c.as_str(),
    }
}

#[derive(Serialize, Deserialize)]
struct Items {
    project: String,
    article: String,
    granularity: String,
    timestamp: String,
    access: String,
    agent: String,
    views: usize,
}

#[derive(Serialize, Deserialize)]
struct PageViewsResult {
    items: Vec<Items>,
}

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
struct EntitiesResult {
    entities: HashMap<String, EntityData>,
}

fn get_page_views(wikipedia: &str, client: &Client) -> Option<usize> {
    // thread::sleep(time::Duration::from_millis(100));

    let res = client
        .get(format!(
            "https://wikimedia.org/api/rest_v1/metrics/pageviews/per-article/en.wikipedia/all-access/all-agents/{}/monthly/2021010100/2021020100",
            wikipedia.replace(" ", "_")
        ))
        .header(USER_AGENT, "wiki-game analysis by wiki-game@tomjwatson.com")
        .send();

    if res.is_err() {
        println!("Got an error from page views API");
        dbg!(&res);
        return None;
    }

    let res = res.unwrap();

    if res.status() == 429 {
        println!("Rate limited by page views API, waiting 30 seconds");
        thread::sleep(time::Duration::from_secs(30));
    }

    let json = res.json::<PageViewsResult>();

    if json.is_err() {
        println!("Can't parse page views results");
        return None;
    }

    let json = json.unwrap();

    if json.items.len() == 0 {
        println!("Empty items returned from page views API");
        return None;
    }

    return Some(json.items[0].views);
}

fn get_item_label(
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

    if res.is_err() {
        println!("Got an error from wikidata API");
        dbg!(&res);
        return None;
    }

    let res = res.unwrap();

    if res.status() == 429 {
        println!("Rate limited by wikidata API, waiting 30 seconds");
        thread::sleep(time::Duration::from_secs(30));
    }

    let json = res.json::<EntitiesResult>();

    if json.is_err() {
        println!("Can't parse wikidata results");
        return None;
    }

    let json = json.unwrap();

    let entity = json.entities.get(id);

    if entity.is_none() {
        println!("Couldn't find entity in wikidata results");
        return None;
    }

    let label = entity.unwrap().labels.get("en");

    if label.is_none() {
        println!("Couldn't find en label in wikidata results");
        return None;
    }

    let label = label.unwrap().value.to_string();

    id_label_map.insert(id.to_string(), label.to_string());

    return Some(label);
}

// fn get_content_length(wikipedia: &str, client: &Client) -> Option<usize> {
//     let res = client
//         .head(format!(
//             "https://en.wikipedia.org/wiki/{}",
//             wikipedia.replace(" ", "_")
//         ))
//         .header(USER_AGENT, "wiki-game analysis by tom@tomjwatson.com")
//         .send();
//
//     if res.is_err() {
//         dbg!(&res);
//         return None;
//     }
//
//     let res = res.unwrap();
//     let content_length = &res.headers().get("content-length");
//
//     if content_length.is_none() {
//         dbg!(res);
//         println!("No content-length header");
//         return None;
//     }
//
//     let content_length = content_length.unwrap().to_str().unwrap().parse::<usize>();
//
//     if content_length.is_err() {
//         dbg!(&content_length);
//         return None;
//     }
//
//     Some(content_length.unwrap())
// }

fn process_item_json(
    item_json: &str,
    date_props: &HashMap<&str, &str>,
    id_label_map: &mut HashMap<String, String>,
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
        r"flag of",
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
            // thread 'main' panicked at 'range end index 1 out of range for slice of length 0', src/main.rs:379:13
            ids.into_iter()
                .map(|id| return get_item_label(id.as_str().unwrap(), id_label_map, &client))
                .filter(|label_option| return label_option.is_some())
                .map(|label_option| return label_option.unwrap())
                .collect(),
        ),
        _ => None,
    };

    if instance_of.is_some() {
        if instance_of
            .clone()
            .unwrap()
            .contains(&String::from("taxon"))
        {
            // println!("Ignore taxon instances");
            return None;
        }
    }

    let occupations: Option<Vec<String>> = match v["claims"]["P106"].as_array() {
        Some(ids) => Some(
            ids.into_iter()
                .map(|id| return get_item_label(id.as_str().unwrap(), id_label_map, &client))
                .filter(|label_option| return label_option.is_some())
                .map(|label_option| return label_option.unwrap())
                .collect(),
        ),
        _ => None,
    };

    let types = if occupations.is_some() {
        occupations.unwrap()
    } else if instance_of.is_some() {
        instance_of.unwrap()
    } else {
        vec![]
    };

    let num_sitelinks = v["sitelinks"].as_object().unwrap().keys().len();

    if num_sitelinks < 15 {
        // println!("Not enough sitelinks");
        return None;
    }

    let page_views = get_page_views(&wikipedia, client);

    if page_views.is_none() {
        println!(
            "Can't fetch page views (https://en.wikipedia.org/wiki/{})",
            wikipedia.replace(" ", "_")
        );
        return None;
    }

    let page_views = page_views.unwrap();

    if page_views < 30000 {
        println!(
            "Not enough page views (https://en.wikipedia.org/wiki/{} = {})",
            wikipedia.replace(" ", "_"),
            page_views
        );
        return None;
    }

    Some(Item {
        id,
        label,
        description,
        image,
        date,
        date_prop_id,
        wikipedia,
        num_sitelinks,
        types,
        page_views,
    })
}

fn main() {
    let mut count: usize = 0;
    let mut seen_count: usize = 0;

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
        ("P7125", "date of the latest one"),
        ("P7124", "date of the first one"),
    ]
    .iter()
    .cloned()
    .collect();

    // File hosts must exist in current path before this produces output
    let lines = read_lines("./processed.json").unwrap();

    let client = Client::builder().build().unwrap();
    let mut id_label_map: HashMap<String, String> = HashMap::new();

    let total: usize = 47711555;

    let path = Path::new("items.json");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why),
        Ok(file) => file,
    };

    for line in lines {
        seen_count += 1;
        if let Ok(item_json) = line {
            if let Some(item) =
                process_item_json(&item_json, &date_props, &mut id_label_map, &client)
            {
                count += 1;
                println!(
                    "Count={}  Seen={}  Total={}  Percent={}  ID Map={}",
                    count,
                    seen_count,
                    total,
                    seen_count / total * 100,
                    id_label_map.len(),
                );
                println!("");
                println!("{}", &item.id);
                println!("{}", &item.label);
                println!("{}", &item.description);
                if let Some(img) = &item.image {
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
                println!("page_views: {}", &item.page_views);
                dbg!(&item.types);
                println!("");

                let json = serde_json::to_string(&item).unwrap();

                match file.write(format!("{}\n", json).as_bytes()) {
                    Err(why) => panic!("couldn't write to {}: {}", display, why),
                    Ok(_) => (),
                }
            }
        }
    }

    println!("Total: Count={} Seen={}", count, seen_count);
}
