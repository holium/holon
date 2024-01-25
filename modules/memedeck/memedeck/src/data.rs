use serde::{Serialize, Deserialize};
use lazy_static::lazy_static;

#[derive(Serialize, Deserialize)]
pub struct Category {
    pub name: String,
    pub count: u32,
}

lazy_static! {
    pub static ref CATEGORIES: Vec<Category> = vec![
        Category { name: "shizo".to_string(), count: 121 },
        Category { name: "epstein".to_string(), count: 34 },
        Category { name: "e/acc".to_string(), count: 30 },
        Category { name: "decels".to_string(), count: 5 },
        Category { name: "trump".to_string(), count: 4 },
    ];
}

// Array of meme image paths
pub static MEMES: &[&str] = &[
    "/airbnb.jpg",
    "/blimps.jpeg",
    "/cat.png",
    "/wizard.jpeg",
];