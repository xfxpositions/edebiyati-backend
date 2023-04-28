use serde::{Deserialize, Serialize};
use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, Document};
use super::Tag;
use super::permissions::Permission;
use chrono::serde::ts_seconds::deserialize as from_ts;
use chrono::{DateTime, Utc};
use std::time::Duration;
use scraper::{Html, Selector};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Content{
    pub html:String,
    pub markdown:String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Comment{
    author_id:String, //anon or user.id
    content:String, //max 250 characters
    id: String,
}

impl Comment{
    pub fn new(mut author_name:Option<String>, content:String)->Comment{
        let mut name = author_name.unwrap_or("anon".to_string()); 
        let id = Uuid::new_v4();
        Comment{
            author_id: name,
            content: content,
            id: id.to_string()
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    // Parse the HTML string

#[derive(Debug, Serialize, Deserialize)]
pub struct Post{
    #[serde(rename = "_id", default)]
    pub id: ObjectId,
    pub title:String,
    pub author:String, // user.id
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
    pub image: String, //optional, url of image    
    pub content: Content,
    pub likes: u32,
    pub dislikes: u32,
    pub views:u32,
    pub status:PostStatus,
    pub tags: Vec<String>,
    pub read_time: u32 // in minutes, for example => 5 = 5 Minutes 
}
impl Post{
    pub fn new(title:String, author:String, image:String, content:Content, status:PostStatus, tags:Vec<String>, read_time:u32) -> Post{
        let now = Utc::now();

        Post{
            id: ObjectId::new(),
            title:title,
            author: author, // id of author
            image: image,
            read_time: read_time,
            content:content,
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
