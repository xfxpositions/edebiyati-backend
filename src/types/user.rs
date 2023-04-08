use mongodb::bson::oid::ObjectId;
use super::permissions::Permission;
use super::post::Post;

use chrono::{DateTime, Utc};

pub struct User{
    pub name: String,
    pub password: String,
    pub email: String,
    pub forgot_mail: String,
    pub permission: Permission,
    pub posts: Vec<Post>,
    pub id: ObjectId,
    pub created_at: DateTime<Utc>,
    pub update_at: DateTime<Utc>,
    pub registred_via: String, // "google" or "email"
    pub avatar: String, // aws s3 object link
    pub view_list: Vec<String>, //Vec<blog.id>
    pub likes: Vec<String>,    // Vec<blog.id>
    pub dislikes: Vec<String>, // Vec<blog.id>
    pub favorites: Vec<String> // Vec<blog.id>
}
