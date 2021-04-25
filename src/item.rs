use serde::Serialize;

pub mod process;

#[derive(Serialize)]
pub struct Item {
    pub date_prop_id: String,
    pub description: String,
    pub id: String,
    pub image: String,
    pub label: String,
    pub page_views: usize,
    pub types: Vec<String>,
    pub wikipedia_title: String,
    pub year: i64,
}
