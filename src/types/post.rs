use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, Document};
use super::Tag;
use super::permissions::Permission;
use chrono::serde::ts_seconds::deserialize as from_ts;
use chrono::{DateTime, Utc};
use std::time::Duration;
use scraper::{Html, Selector};

#[derive(Debug, Serialize, Deserialize)]
pub struct Content{
    html:String,
    markdown:String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment{
    author_name:String, //anon or user.id
    content:String, //max 250 characters
    author_avatar: String //profile image of author, give a guest avatar for anon
}

impl Comment{
    pub fn new(anon:bool, mut author_name:Option<String>, content:String, mut author_avatar: String)->Comment{
        let mut name = author_name.unwrap_or("anon".to_string()); 
        if anon{
            name = "anon".to_string();
            author_avatar = "https://upload.wikimedia.org/wikipedia/commons/1/16/K2-big.jpg".to_string();
        }
        Comment{
            author_name: name,
            content: content,
            author_avatar: author_avatar
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PostStatus{
   Public,
   Private,
   OnlyFriends,
   Deleted
}
// impl PostStatus{
//     pub fn to_string(){
//         match self{
//             Self::Public => "public",
//             Self::Private => "private",
//             Self::OnlyFriends=> "friends",
//             Self::Deleted => "deleted",

//         };
//     }
// }
fn calculate_read_time(html: &str) -> u32 {
    // Parse the HTML string
    let document = Html::parse_document(html);

    // Use a scraper selector to find all text nodes
    let selector = Selector::parse("*[class]").unwrap();
    let text_nodes = document.select(&selector);

    // Extract the text content and count the number of words
    let mut num_words = 0;
    for text_node in text_nodes {
        let text = text_node.text().collect::<String>();
        num_words += text.split_whitespace().count();
    }

    // Calculate the estimated reading time
    let words_per_minute = 200; // assuming 200 words per minute
    let minutes = num_words as f64 / words_per_minute as f64;
    let read_time_minutes = minutes.ceil() as u32;
    read_time_minutes
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Post{
    pub title:String,
    pub author:String, // user.id
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
    pub image: String, //optional, url of image    
    pub content: Content,
    pub comments: Vec<Comment>,
    pub likes: u32,
    pub dislikes: u32,
    pub views:u32,
    pub status:PostStatus,
    pub tags: Vec<Tag>,
    pub read_time: u32 // in minutes, for example => 5 = 5 Minutes 
}
impl Post{
    pub fn new(title:String, author:String, image:Option<String>, content:Content, comments:Vec<Comment>, status:PostStatus, tags:Vec<Tag>,read_time:Option<u32>) -> Post{
        let now = Utc::now();

        Post{
            title:title,
            author: author,
            image: image.unwrap_or("https://hips.hearstapps.com/hmg-prod/images/cherry-blossom-facts-1578344148.jpg".to_string()),
            read_time: read_time.unwrap_or(calculate_read_time(&content.html)),
            content:content,
            comments: comments,
            likes:0,
            dislikes:0,
            views:0,
            status:status,
            tags:tags,
            updated_at: now,
            created_at: now
        }
    }
}
