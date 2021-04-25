use serde::Serialize;

pub mod process;

#[derive(Serialize)]
pub struct Item {
    pub date_prop_id: String,
    pub description: String,
    pub id: String,
    pub image: String,
    pub instance_of: Vec<String>,
    pub label: String,
    pub occupations: Option<Vec<String>>,
    pub page_views: usize,
    pub wikipedia_title: String,
    pub year: i64,
}
