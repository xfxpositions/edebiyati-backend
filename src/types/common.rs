use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Common{
    pub total_view: u32,
    pub total_clicked:u32, 
    pub user_count: u32,
    pub post_count: u32,
    pub tag_count: u32
}