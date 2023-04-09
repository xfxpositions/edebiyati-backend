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
            Self::Banned => "GET".to_string(),
            Self::Guest => "POST".to_string(),
            Self::Author => "PUT".to_string(),
            Self::Admin => "DELETE".to_string(),
        }
    }
}