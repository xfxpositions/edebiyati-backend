use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]

pub struct Tag{
    pub name: String,
    used_by: Vec<String> // post id's
}

impl Tag{
    pub fn new(name:String) -> Tag{
        Tag{
            name: name,
            used_by: vec![]
        }
    }
    pub fn add(&mut self, post_id: String){
        self.used_by.push(post_id);
    }
}
