use std::{str::FromStr, io::Write, collections::HashMap};

use actix_web::{web::{self}, HttpResponse, Responder};
use actix_multipart::Multipart;

use mongodb::{Database, bson::{self, doc, from_document, oid::ObjectId}, options::{FindOneAndUpdateOptions, ReturnDocument}, Collection};
use serde::Deserialize;
use serde_json::json;
use sha256::digest;

use crate::{types::Post, utils::upload_image_to_s3, types::Content, types::{PostStatus, Comment}, types::Tag, types::DEFAULT_POST_IMAGE};
use crate::utils::sign_jwt;
use crate::utils::calculate_reading_time;
use futures::{StreamExt, TryStreamExt};
use uuid::Uuid;


#[derive(Deserialize, Clone)]
struct CreatePostRequest {
    title: String,
    author: String,
    image: Option<String>,
    content: Content,
    status: PostStatus,
    tags: Vec<String>,
    read_time: Option<u32>
}

async fn create_post( post_req: web::Json<CreatePostRequest>, db: web::Data<Database>)->impl Responder {
    let mut post = post_req.clone();
    let image = post.image.take().unwrap_or(DEFAULT_POST_IMAGE.to_string());
    let content_html = &post.content.html;
    let reading_time =  calculate_reading_time(&content_html);
    let new_post = Post::new(post.title, post.author, post.image, post.content, post.status, post.tags, reading_time as u32 );
    let post_doc = bson::to_document(&new_post).unwrap();
    println!("POST DOC!, {}",post_doc);
    let result = db.collection("posts").insert_one(post_doc, None).await;

    match result {
        Ok(post) => HttpResponse::Ok().json(json!({"Post":post})),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating post: {}", e))
    }
}
async fn fetch_post_by_id(Post_id: web::Path<String>, db: web::Data<Database>) -> impl Responder {

    let collection = db.collection("posts");

    fn is_valid_objectid(id: &str) -> bool {
        if let Ok(_) = ObjectId::from_str(id) {
            true
        } else {
            false
        }
    }

    if !is_valid_objectid(&Post_id) {
        return HttpResponse::BadRequest().json(json!({"error":"Invalid Post ID"}));
    }


    let id = ObjectId::from_str(&Post_id).unwrap();    
    match collection.find_one(doc! {"_id": id}, None).await {
        Ok(result) => {
            if let Some(doc) = result {
                // Deserialize the Post document to a Post struct
                let Post: Post = from_document(doc).unwrap();
                // Return the Post as a JSON response
                HttpResponse::Ok().json(Post)
            } else {
                // Return a 404 Not Found error as a JSON response
                HttpResponse::NotFound().json(json!({"error":"Post not found"}))
            }
        }
        Err(error) => {
            // Return an error message as a JSON response
            HttpResponse::InternalServerError().json(format!("Failed to fetch Post: {}", error))
        }
    }
}


async fn update_post(
    Post_id: web::Path<String>,
    new_data: web::Json<HashMap<String, String>>,
    db: web::Data<Database>
) -> impl Responder {
    let collection = db.collection::<Post>("Posts");

    fn is_valid_objectid(id: &str) -> bool {
        if let Ok(_) = ObjectId::from_str(id) {
            true
        } else {
            false
        }
    }

    if !is_valid_objectid(&Post_id) {
        return HttpResponse::BadRequest().json(json!({"error":"Invalid Post ID"}));
    }

    let id = ObjectId::from_str(&Post_id).unwrap();

    let mut update_fields = doc! {};

    for (key, value) in new_data.iter() {
        update_fields.insert(key, value);
    }

    let update_doc = doc! {"$set": update_fields};

    let result = collection
        .update_one(doc! {"_id": id}, update_doc, None)
        .await;

    match result {
        Ok(Post) => HttpResponse::Ok().json(json!({"Post": Post})),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating Post: {}", e))
    }
}


#[derive(Deserialize, Clone)]
struct AddCommentRequest{
    author_id: Option<String>,
    content: String
}
async fn add_comment(Post_id: web::Path<String>, comment_data: web::Json<AddCommentRequest>, db: web::Data<Database>)-> impl Responder{

    let collection = db.collection::<Post>("posts");
    fn is_valid_objectid(id: &str) -> bool {
        if let Ok(_) = ObjectId::from_str(id) {
            true
        } else {
            false
        }
    }
    if !is_valid_objectid(&Post_id) {
        return HttpResponse::BadRequest().json(json!({"error":"Invalid Post ID"}));
    }
    if comment_data.content.clone().len() > 250 || comment_data.content.clone().len() <=1 {
        return HttpResponse::BadRequest().json(json!({"error":"comment content must be between 250 and 1 characters"}));
    }
    let post_id = ObjectId::from_str(&Post_id).unwrap();    
    let comment = Comment::new(comment_data.author_id.clone(), comment_data.content.clone());
    let filter = doc! {"_id": post_id};
    let doc = bson::to_document(&comment).unwrap();
    let update = doc! {"$push": {"comments": doc}};
    let result = collection.update_one(filter, update, None).await;
    
    match result{
        Ok(_)=>{HttpResponse::Ok().json(json!({"success":"comment added!"}))},
        Err(error) => {
            // Return an error message as a JSON response
            HttpResponse::InternalServerError().json(format!("Failed to fetch Post: {}", error))
        }
    }


}

pub fn post_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/post/create")
            .route(web::post().to(create_post))
    )
    .service(
        web::resource("/post/fetch/{id}")
            .route(web::get().to(fetch_post_by_id))
    )
    .service(
        web::resource("/post/update/{id}")
            .route(web::post().to(update_post))
    )
    .service(
        web::resource("/post/addcomment/{id}")
            .route(web::post().to(add_comment))
    );
}
