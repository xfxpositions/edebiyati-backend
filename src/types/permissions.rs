use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum Permission{
    Banned,
    Guest,
    Author,
    Admin
}