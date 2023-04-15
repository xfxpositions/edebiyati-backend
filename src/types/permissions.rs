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
            Self::Banned => "Banned".to_string(),
            Self::Guest => "Guest".to_string(),
            Self::Author => "Author".to_string(),
            Self::Admin => "Admin".to_string(),
        }
    }
}