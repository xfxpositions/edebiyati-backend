use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Permission{
    Banned,
    Guest,
    Author,
    Admin
}
impl Permission{
    pub fn to_string(&self)->String{
        match self {
            Self::Banned => "banned".to_string(),
            Self::Guest => "guest".to_string(),
            Self::Author => "author".to_string(),
            Self::Admin => "admin".to_string(),
        }
    }
}