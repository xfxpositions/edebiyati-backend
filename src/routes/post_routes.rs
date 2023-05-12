use std::{str::FromStr, io::Write, collections::HashMap};

use actix_web::{web::{self}, HttpResponse, Responder};
use actix_multipart::Multipart;
use std::collections::HashSet;
use mongodb::{Database, bson::{self, doc, from_document, oid::ObjectId, Regex, Document, document, Bson}, options::{FindOneAndUpdateOptions, ReturnDocument, FindOptions, FindOneOptions}, Collection};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{types::Post, utils::upload_image_to_s3, types::Content, types::{PostStatus, Comment}, types::Tag, types::{DEFAULT_POST_IMAGE, User}};

use crate::utils::calculate_reading_time;
use futures::{StreamExt, TryStreamExt};





async fn upload_image(mut payload: Multipart) -> impl Responder {

    // Read the image data from the multipart payload
    if let Some(mut field) = payload.try_next().await.unwrap() {
        let content_type = field.content_type();
        if let Some(mime) = content_type {
            if mime.type_() == mime::IMAGE {
                let mut file_bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    let data = chunk.unwrap();
                    file_bytes.write_all(&data).unwrap();
                }

                // Upload the image to S3
                let uuid = Uuid::new_v4();
                let key = format!("{}.png",uuid.to_string()); 
                match upload_image_to_s3(&key, &file_bytes).await {
                    Ok(url) => {
                        let image_url = url;
                        HttpResponse::Ok().json(json!({
                            "message":"Image uploaded successfuly",
                            "url":image_url
                        }))
                    },
                    Err(e) => HttpResponse::InternalServerError().body(e.to_string()),
                }
            } else {
                HttpResponse::UnsupportedMediaType().body("Only image files are supported")
            }
        } else {
            HttpResponse::UnsupportedMediaType().body("Invalid content type")
        }
    } else {
        HttpResponse::BadRequest().body("No image file found in request payload")
    }
}

#[derive(Deserialize, Clone)]
struct CreatePostRequest {
    title: String,
    author: String,
    image: String,
    content: Content,
    status: PostStatus,
    tags: Vec<String>,
}

async fn create_post( post_req: web::Json<CreatePostRequest>, db: web::Data<Database>)->impl Responder {
    fn is_valid_objectid(id: &str) -> bool {
        if let Ok(_) = ObjectId::from_str(id) {
            true
        } else {
            false
        }
    }

    if !is_valid_objectid(&post_req.author.clone()) {
        return HttpResponse::BadRequest().json(json!({"error":"Invalid Post ID"}));
    }

    let user_collection = db.collection::<User>("users");

    let user_id = ObjectId::from_str(&post_req.author.clone()).unwrap();    

    let user_filter = doc! {"_id": user_id};
    

    let user = match user_collection.find_one(user_filter.clone(), None).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            return HttpResponse::BadRequest().body(format!(
                "Error creating post: user with id {} not found",
                &post_req.author
            ))
        }
        Err(e) => {
            return HttpResponse::InternalServerError().body(format!(
                "Error creating post: unable to query user: {}",
                e
            ))
        }
    };



    let mut post = post_req.clone();
    let content_html = &post.content.html;
    let reading_time =  calculate_reading_time(&content_html);
    let new_post = Post::new(post.title, post.author, post.image, post.content, post.status, post.tags, reading_time as u32);
    let post_doc = bson::to_document(&new_post).unwrap();
    println!("POST DOC!, {}",post_doc);
    let result = db.collection("posts").insert_one(post_doc, None).await;

    match result {
        Ok(post) => {
            let post_id_str = post.inserted_id.as_object_id().unwrap().to_hex();
           
            let user_update = doc! {"$push": {"posts": post_id_str}};
            let update_result = user_collection.update_one(user_filter, user_update, None).await;
            if let Err(e) = update_result {
                return HttpResponse::BadRequest().body(format!("Error updating user: {}", e));
            }
            HttpResponse::Ok().json(json!({"Post": post}))
        },
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating post: {}", e))
    }
}
#[derive(Debug, Deserialize)]
struct FetchOptions {
    fields: Option<Vec<String>>,
}

impl AsRef<FetchOptions> for FetchOptions {
    fn as_ref(&self) -> &FetchOptions {
        self
    }
}

async fn fetch_post_by_id(
    Post_id: web::Path<String>,
    query: web::Query<HashMap<String, String>>,
    db: web::Data<Database>,
) -> impl Responder {
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

    let mut options = FindOneOptions::default();

    // If the `fields` field is present in the query string,
    // create a projection document to fetch only the specified fields.
    if let Some(fields) = query.get("fields") {
        let field_vec: Vec<String> = fields.split(",").map(|s| s.to_owned()).collect();
        let mut projection = doc! {};
        for field in field_vec {
            projection.insert(field, 1);
        }
        options.projection = Some(projection);
    }

    match collection.find_one(doc! {"_id": id}, options).await {
        Ok(result) => {
            if let Some(doc) = result {
                // Deserialize the Post document to a Post struct
                let document: serde_json::Value  = from_document(doc).unwrap();
                let post_json: Value = serde_json::to_value(document).unwrap();

                // Return the Post as a JSON response
                HttpResponse::Ok().json(post_json)
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


#[derive(Debug, Deserialize)]
struct SearchParams {
    title: Option<String>,
    author: Option<String>,
    content: Option<String>,
    date: Option<i64>,
}


async fn fetch_all(params: web::Query<SearchParams>,page: web::Path<i32>, db: web::Data<Database>) -> impl Responder {
    
    let mut query = doc! {};

    if let Some(title) = &params.title {
        let regex_str = format!(".*{}.*", title);
        query.insert("title", doc! { "$regex": regex_str, "$options": "i" });
    }

    if let Some(content) = &params.content {
        let regex_str = format!(".*{}.*", content);

        query.insert("content.html", doc! { "$regex": regex_str, "$options": "i" });
    }

    if let Some(author) = &params.author {
        query.insert("author", author);
    }

    if let Some(date) = &params.date {
        query.insert("created_at", *date);
        query.insert("updated_at", *date);
    }

    println!("query {:?}",query);

    // Define the number of documents to skip and the number of documents to return
    let page_size = 5;

    let collection = db.collection::<Post>("posts");
    let allah = collection.find(query, None).await.unwrap();
    let skip_size = (page.into_inner() - 1) * page_size;
    let posts: Vec<Post> = allah
        .skip((skip_size as u64).try_into().unwrap())
        .take((page_size as u64).try_into().unwrap())
        .filter_map(|result| async {
            match result {
                Ok(post) => Some(post),
                Err(_) => None,
            }
        })
        .collect().await;

    HttpResponse::Ok().json(posts)
}

async fn update_post(
    user_id: web::Path<String>,
    new_data: web::Json<HashMap<String, Value>>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Post>("posts");

    fn is_valid_objectid(id: &str) -> bool {
        ObjectId::from_str(id).is_ok()
    }

    if !is_valid_objectid(&user_id) {
        return HttpResponse::BadRequest().json(json!({"error":"Invalid post ID"}));
    }

    let id = ObjectId::from_str(&user_id).unwrap();

    let mut update_fields = doc! {};

    for (key, value) in new_data.iter() {
        if key.contains('.') {
            let mut parts = key.split('.').rev();
            let last_key = parts.next().unwrap();
    
            // Check if the last key is an array element
            if last_key.starts_with('[') && last_key.ends_with(']') {
                let array_key = &last_key[1..last_key.len() - 1];
                let mut array_doc = doc! {};
                if let Ok(index) = array_key.parse::<usize>() {
                    // If the array element is an integer index, use the $ positional operator
                    let mut array_update = doc! {};
                    array_update.insert("$", bson::to_bson(value).unwrap_or(Bson::Null));
                    array_doc.insert(index.to_string(), array_update);
                    let mut doc = doc! {};
                    for part in parts {
                        let mut new_doc = doc! {part: doc};
                        doc = new_doc;
                    }
                    doc.insert(last_key.to_string(), array_doc);
                    update_fields.insert(key.to_string(), doc);
                } else {
                    // If the array element is a string key, update the entire array element
                    array_doc.insert(last_key.to_string(), bson::to_bson(value).unwrap_or(Bson::Null));
                    let mut doc = doc! {};
                    for part in parts {
                        let mut new_doc = doc! {part: doc};
                        doc = new_doc;
                    }
                    doc.insert(array_key.to_string(), array_doc);
                    update_fields.insert(key.to_string(), doc);
                }
            } else {
                // If the last key is not an array element, update the entire field
                update_fields.insert(key.to_string(), bson::to_bson(value).unwrap_or(Bson::Null));
            }
        } else {
            // If the key does not contain a dot, update the entire field
            update_fields.insert(key.to_string(), bson::to_bson(value).unwrap_or(Bson::Null));
        }
    }
    
    println!("update => {:?}",update_fields);
    let update_doc = doc! {"$set": update_fields};
    let result = collection.update_one(doc! {"_id": id}, update_doc, None).await;

    match result {
        Ok(user) => HttpResponse::Ok().json(json!({"user": user})),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error updating post: {}", e)),
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
    if comment_data.content.clone().len() > 400 || comment_data.content.clone().len() <=1 {
        return HttpResponse::BadRequest().json(json!({"error":"comment content must be between 250 and 1 characters"}));
    }
    let post_id = ObjectId::from_str(&Post_id).unwrap();    
    let comment = Comment::new(comment_data.author_id.clone(), comment_data.content.clone());
    let filter = doc! {"_id": post_id};
    let doc = bson::to_document(&comment).unwrap();
    let update = doc! {"$push": {"comments": doc}};
    let result = collection.update_one(filter, update, None).await;
    
    match result{
        Ok(result)=>{HttpResponse::Ok().json(json!({"success":format!("{:?}",result)}))},
        Err(error) => {
            // Return an error message as a JSON response
            HttpResponse::InternalServerError().json(format!("Failed to fetch Post: {}", error))
        }
    }

}

async fn add_reply(
    query: web::Query<HashMap<String, String>>,
    post_id: web::Path<String>,
    comment_data: web::Json<AddCommentRequest>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Post>("posts");

    // Define a function to check if a given ID is a valid ObjectId
    fn is_valid_objectid(id: &str) -> bool {
        ObjectId::from_str(id).is_ok()
    }

    // Check if the post ID is a valid ObjectId
    if !is_valid_objectid(&post_id) {
        return HttpResponse::BadRequest().json(json!({"error": "Invalid Post ID"}));
    }

    // Check if the comment content is between 1 and 400 characters
    if comment_data.content.len() <= 1 || comment_data.content.len() > 400 {
        return HttpResponse::BadRequest().json(json!({"error": "Comment content must be between 1 and 400 characters"}));
    }

    let post_id = ObjectId::from_str(&post_id).unwrap();
    let new_comment = Comment::new(comment_data.author_id.clone(), comment_data.content.clone());
    let filter = doc! {"_id": post_id};
    let post = collection.find_one(filter.clone(), None).await;

    // Check if the post exists
    let mut post = match post {
        Ok(Some(post)) => post,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Post not found"})),
        Err(error) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to fetch Post: {}", error)})),
    };

    // Find the comment with matching ID in the comments vector
    let comment_id = query.get("comment_id").unwrap();
    let comment = post.comments.iter_mut().find(|c| c.id == comment_id.to_string());

    // Check if the comment exists
    let comment = match comment {
        Some(comment) => comment,
        None => return HttpResponse::NotFound().json(json!({"error": "Comment not found"})),
    };

    // Add the new comment as a reply to the existing comment
    comment.replies.push(new_comment);

    // Update the post in the database with the new comment
    let doc = bson::to_document(&post).unwrap();
    let update = doc! {"$set": {"comments": doc.get("comments").unwrap()}};
    let result = collection.update_one(filter, update, None).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"success": "Reply added!"})),
        Err(error) => HttpResponse::InternalServerError().json(json!({"error": format!("Failed to update Post: {}", error)})),
    }
}


#[derive(Deserialize, Clone)]
struct AddLikeRequest{
    user_id: String,
}

async fn add_like(
    query: web::Query<HashMap<String, String>>,
    post_id: web::Path<String>,
    db: web::Data<Database>,
) -> impl Responder {
    let collection = db.collection::<Post>("posts");

    // Define a function to check if a given ID is a valid ObjectId
    fn is_valid_objectid(id: &str) -> bool {
        ObjectId::from_str(id).is_ok()
    }

    // Check if the post ID is a valid ObjectId
    if !is_valid_objectid(&post_id) {
        return HttpResponse::BadRequest().json(json!({"error": "Invalid Post ID"}));
    }

   

    let post_id = ObjectId::from_str(&post_id).unwrap();
    //let new_comment = Comment::new(comment_data.author_id.clone(), comment_data.content.clone());

    let filter = doc! {"_id": post_id};
    let post = collection.find_one(filter.clone(), None).await;

    // Check if the post exists
    let mut post = match post {
        Ok(Some(post)) => post,
        Ok(None) => return HttpResponse::NotFound().json(json!({"error": "Post not found"})),
        Err(error) => return HttpResponse::InternalServerError().json(json!({"error": format!("Failed to fetch Post: {}", error)})),
    };

    // Find the comment with matching ID in the comments vector
    let comment_id = query.get("comment_id").unwrap();
    let user_id = query.get("user_id").unwrap();
    let comment = post.comments.iter_mut().find(|c| c.id == comment_id.to_string());

    // Check if the comment exists
    let comment = match comment {
        Some(comment) => comment,
        None => return HttpResponse::NotFound().json(json!({"error": "Comment not found"})),
    };

    // Add the new comment as a reply to the existing comment
    //comment.replies.push(new_comment);
    let mut is_deleted = false;
    if comment.likes.contains(&user_id.clone()){
        comment.likes.retain(|id| id != user_id);
        is_deleted = true;
    } else {
        comment.likes.push(user_id.clone());
    }
    
    // Update the post in the database with the new comment
    let doc = bson::to_document(&post).unwrap();
    let update = doc! {"$set": {"comments": doc.get("comments").unwrap()}};
    let result = collection.update_one(filter, update, None).await;

    match result {
        Ok(_) => HttpResponse::Ok().json(json!({"success": "Reply added!","isDeleted":is_deleted})),
        Err(error) => HttpResponse::InternalServerError().json(json!({"error": format!("Failed to update Post: {}", error)})),
    }
}

async fn search(params: web::Query<SearchParams>, db: web::Data<Database>) -> HttpResponse {
    let collection = db.collection::<Post>("posts");

    // Build the search query based on the search parameters
    let mut query = doc! {};
    if let Some(title) = &params.title {
        query.insert("title", title);
    }
    if let Some(author) = &params.author {
        query.insert("author", author);
    }
    if let Some(content) = &params.content {
        query.insert("content", content);
    }
    if let Some(date) = &params.date {
        query.insert("date", *date);
    }

    let mut cursor = collection.find(query, None).await.unwrap();
    
    let posts: Vec<Post> = cursor
    .filter_map(|result| async {
        match result {
            Ok(post) => Some(post),
            Err(_) => None,
        }
    })
    .collect().await;

    HttpResponse::Ok().json(posts)


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
    )
    .service(
        web::resource("/post/addreply/{id}")
            .route(web::post().to(add_reply))
    )
    .service(
        web::resource("/post/fetchall/{page}")
            .route(web::get().to(fetch_all))
    )
    .service(
        web::resource("/post/search")
            .route(web::get().to(search))
    )
    .service(
        web::resource("/post/upload_image")
            .route(web::post().to(upload_image))
    )
    .service(
        web::resource("/post/add_like/{id}")
            .route(web::post().to(add_like))
    );
}
