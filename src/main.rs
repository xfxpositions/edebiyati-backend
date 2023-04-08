use std::str::FromStr;

use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use mongodb::{bson::{doc, Document, oid::ObjectId, from_document}, options::ClientOptions, Client, error::Error as MongoError};
use serde::{Deserialize, Serialize};
mod types;
mod routes;
use routes::{post_routes, user_routes};
use types::{Common,Permission,Post,Tag,User};



#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .configure(post_routes)
            .configure(user_routes)
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
