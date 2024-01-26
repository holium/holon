use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

#[derive(Serialize, Deserialize)]
pub struct UploadData {
    pub url: String,
}

#[derive(Serialize, Deserialize)]
pub struct MemeCategory {
    pub name: String,
    pub count: u32,
}

lazy_static! {
    pub static ref MEME_CATEGORIES: Vec<MemeCategory> = vec![
        MemeCategory { name: "shizo".to_string(), count: 121 },
        MemeCategory { name: "epstein".to_string(), count: 34 },
        MemeCategory { name: "e/acc".to_string(), count: 30 },
        MemeCategory { name: "decels".to_string(), count: 5 },
        MemeCategory { name: "trump".to_string(), count: 4 },
    ];
}

#[derive(Serialize, Deserialize)]
pub struct MemeTemplate {
    pub name: String,
    pub count: u32,
}

lazy_static! {
    pub static ref MEME_TEMPLATES: Vec<MemeTemplate> = vec![
        MemeTemplate { name: "bell curve".to_string(), count: 32 },
        MemeTemplate { name: "distracted boyfriend".to_string(), count: 12 },
        MemeTemplate { name: "expanding brain".to_string(), count: 16 },
        MemeTemplate { name: "anakin padme 4 panel".to_string(), count: 4 },
        MemeTemplate { name: "two buttons".to_string(), count: 3 },
    ];
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Meme {
    pub id: String,
    pub url: String,
}

lazy_static! {
    pub static ref MEMES: Vec<Meme> = vec![
        Meme { id: "airbnb".to_string(), url: "/airbnb.jpg".to_string() },
        Meme { id: "blimps".to_string(), url: "/blimps.jpeg".to_string() },
        Meme { id: "cat".to_string(), url: "/cat.png".to_string() },
        Meme { id: "wizard".to_string(), url: "/wizard.jpeg".to_string() },
    ];
}
