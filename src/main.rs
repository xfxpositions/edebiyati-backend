use std::str::FromStr;

use actix_web::{web::{self, Data}, App, HttpServer, http::header::{HeaderMap}};
use mongodb::{Client, Database, bson::{self, Document, to_document, doc}, Collection};
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
    // Check if a document exists in the collection and create a new one if it doesn't
    let coll: Collection<Document> = db.collection("common");

    let count = coll.count_documents(doc! {}, None).await.unwrap();

    if count == 0 {
        let common = Common {
            total_view: 0,
            total_clicked: 0,
            user_count: 0,
            post_count: 0,
            tag_count: 0
        };
        let doc = to_document(&common).unwrap();
        let _ = coll.insert_one(doc, None).await;
    }
    fn jwt_middleware(headers:HeaderMap){
        //println!("hello from jwt_middleware");
        // for header in headers.iter(){
        //     println!("Header: , {:?}", header);
        // }
        // let auth_header = headers.get("authorization");
        // println!("auth Header: , {:?}", auth_header);

        
    }
    HttpServer::new(move || {
        println!("server listening in 8080");
        App::new()
             .wrap_fn(|req, srv| {
            println!("Hi from start. You requested: {}", req.path());
            let headers = req.headers();
            jwt_middleware(headers.clone());
            srv.call(req).map(move |res: Result<ServiceResponse, Error>| {
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
