use std::str::FromStr;

use actix_web::{web::{self, Data}, App, HttpResponse, HttpServer, Responder, dev::WebService, HttpRequest};
use mongodb::{Client, Database};
use serde::{Deserialize, Serialize};
mod types;
mod routes;
use routes::{post_routes, user_routes};
mod utils;
use types::{Common,Permission,Post,Tag,User};
use futures_util::future::FutureExt;
use actix_web::{dev::Service as _ };
use actix_web::{dev::ServiceRequest, dev::ServiceResponse, Error};
use futures_util::future::{Future, Ready};


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let client = Client::with_uri_str("mongodb://localhost:27017").await.unwrap();
    let db: Database = client.database("mydb");
    
    HttpServer::new(move || {
        App::new()
             .wrap_fn(|req, srv| {
            println!("Hi from start. You requested: {}", req.path());
            srv.call(req).map(|res| {
                println!("Hi from response");
                res
            })
            })       
            .app_data(Data::new(db.clone()))
            .configure(post_routes)
            .configure(user_routes)
    })

    .bind("127.0.0.1:8080")?
    .run()
    .await
}
