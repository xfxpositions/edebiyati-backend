use std::str::FromStr;

use actix_web::{web::{self, Data}, App, HttpResponse, HttpServer, Responder};
use mongodb::{bson::{doc, Document, oid::ObjectId, from_document}, options::ClientOptions, Client, error::Error as MongoError, Database};
use serde::{Deserialize, Serialize};
mod types;
mod routes;
use routes::{post_routes, user_routes};
mod utils;
use types::{Common,Permission,Post,Tag,User};



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await.unwrap();
    let db: Database = client.database("mydb");
    
    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(db.clone()))
            .configure(post_routes)
            .configure(user_routes)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
