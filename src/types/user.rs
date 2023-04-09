use mongodb::bson::oid::ObjectId;
use mongodb::bson::{self, Document};
use serde::{Deserialize, Serialize};
use super::permissions::Permission;
use super::post::Post;
use chrono::serde::ts_seconds::deserialize as from_ts;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct User{ 
    #[serde(rename = "_id", default)]
    pub id: ObjectId,
    pub name: String,
    pub password: String,
    pub email: String,
    pub forgot_mail: Option<String>,
    pub permission: Permission,
    pub posts: Vec<Post>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
    pub registred_via: String, // "google" or "email"
    pub avatar: Option<String>, // aws s3 object link
    pub view_list: Vec<String>, //Vec<blog.id>
    pub likes: Vec<String>,    // Vec<blog.id>
    pub dislikes: Vec<String>, // Vec<blog.id>
    pub favorites: Vec<String> // Vec<blog.id>
}

impl User {
    pub fn to_document(&self) -> Document {
        bson::to_document(self).unwrap()
    }

    pub fn from_document(doc: Document) -> Result<Self, mongodb::bson::de::Error> {
        bson::from_document(doc)
    }
    pub fn new(name: String, password: String, email: String, forgot_mail: Option<String>) -> Self {
        let now = Utc::now();
        User {
            id: ObjectId::new(),
            name,
            password,
            email,
            forgot_mail: forgot_mail,
            permission: Permission::Guest,
            posts: vec![],
            created_at: now,
            updated_at: now,
            registred_via: String::new(),
            avatar: None,
            view_list: vec![],
            likes: vec![],
            dislikes: vec![],
            favorites: vec![],
        }
    }
}
