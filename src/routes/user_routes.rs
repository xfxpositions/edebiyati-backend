use std::{str::FromStr, io::Write};

use actix_web::{web::{self}, HttpResponse, Responder};
use actix_multipart::Multipart;

use mongodb::{Database, bson::{self, doc, from_document, oid::ObjectId}, options::{FindOneAndUpdateOptions, ReturnDocument}, Collection};
use serde::Deserialize;
use serde_json::json;
use sha256::digest;

use crate::{types::User, utils::upload_image_to_s3};
use crate::utils::sign_jwt;
use futures::{StreamExt, TryStreamExt};
use uuid::Uuid;


#[derive(Deserialize)]
struct CreateUserRequest {
    name: String,
    password: String,
    email: String,
    forgot_mail: Option<String>,
}

async fn create_user(user: web::Json<CreateUserRequest>, db: web::Data<Database>)->impl Responder {
    fn hash_password(password: String) -> String {
        let hashed_password = digest(password);
        hashed_password
    }
    let new_user = User::new(
        user.name.clone(),
        hash_password(user.password.clone()),
        user.email.clone(),
        user.forgot_mail.clone(),
    );

    let user_doc = bson::to_document(&new_user).unwrap();
    let result = db.collection("users").insert_one(user_doc, None).await;

    match result {
        Ok(user) => HttpResponse::Ok().json(json!({"user":user})),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating user: {}", e))
    }
}
async fn fetch_user_by_id(user_id: web::Path<String>, db: web::Data<Database>) -> impl Responder {

    let collection = db.collection("users");

    fn is_valid_objectid(id: &str) -> bool {
        if let Ok(_) = ObjectId::from_str(id) {
            true
        } else {
            false
        }
    }

    if !is_valid_objectid(&user_id) {
        return HttpResponse::BadRequest().json(json!({"error":"Invalid user ID"}));
    }


    let id = ObjectId::from_str(&user_id).unwrap();    
    match collection.find_one(doc! {"_id": id}, None).await {
        Ok(result) => {
            if let Some(doc) = result {
                // Deserialize the user document to a User struct
                let user: User = from_document(doc).unwrap();
                // Return the user as a JSON response
                HttpResponse::Ok().json(user)
            } else {
                // Return a 404 Not Found error as a JSON response
                HttpResponse::NotFound().json(json!({"error":"user not found"}))
            }
        }
        Err(error) => {
            // Return an error message as a JSON response
            HttpResponse::InternalServerError().json(format!("Failed to fetch user: {}", error))
        }
    }
}

async fn upload_avatar(user_id: web::Path<String>, db: web::Data<Database>, mut payload: Multipart) -> impl Responder {

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
                let ext = field.content_disposition().disposition.clone();
                // let a = uuid.hyphenated().to_string();
                let filename = field
                    .content_disposition()
                    .get_filename()
                    .unwrap_or("unnamed.file")
                    .to_owned();
                let key = format!("avatar-{}.png",user_id.clone()); 
                
                match upload_image_to_s3(&key, &file_bytes).await {
                    Ok(url) => {
                        let collection: Collection<User> = db.collection("users");

                        fn is_valid_objectid(id: &str) -> bool {
                            if let Ok(_) = ObjectId::from_str(id) {
                                true
                            } else {
                                false
                            }
                        }
                    
                        if !is_valid_objectid(&user_id) {
                            return HttpResponse::BadRequest().json(json!({"error":"Invalid user ID"}));
                        }
                    
                        let image_url = url;
                        let id = ObjectId::from_str(&user_id).unwrap();    
                        let options = FindOneAndUpdateOptions::builder()
                            .return_document(ReturnDocument::After)
                            .build();
                        let filter = doc! {"_id": ObjectId::from_str(user_id.to_string().as_str()).unwrap()};
                        let update = doc! {"$set": {"avatar": image_url.clone()}};
                        match collection.find_one_and_update(filter, update, options).await {
                            Ok(user_doc) => {
                                if let Some(user_doc) = user_doc {
                                    HttpResponse::Ok().body(format!("Avatar uploaded successfully, url {}",image_url))
                                } else {
                                    HttpResponse::NotFound().body("User not found")
                                }
                            },
                            Err(e) => {
                                HttpResponse::InternalServerError().body(e.to_string())
                            }
                        }
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


#[derive(Deserialize)]
struct LoginRequest {
    name: Option<String>,
    password: String,
    email: Option<String>,
}

async fn login(request_user: web::Json<LoginRequest>, db: web::Data<Database>) -> impl Responder {
    let collection = db.collection("users");
    let filter = doc! {
        "$or": [
            {"name": &request_user.name},
            {"email": &request_user.email}
        ]
    };
    match collection.find_one(filter, None).await {
        Ok(result) => {
            if let Some(doc) = result {
                // Deserialize the user document to a User struct
                let user: User = from_document(doc).unwrap();
                // Check if the input password matches the stored password hash
                fn hash_password(password: String) -> String {
                    let hashed_password = digest(password);
                    hashed_password
                }
                let input_password_hash = hash_password(request_user.password.clone());
                if input_password_hash == user.password {
                    // Passwords match, return an OK  response
                    match sign_jwt(user.id.to_string().as_str()) {
                        Ok(token) => {
                            // Passwords match, return an OK response with the JWT token and user object
                            HttpResponse::Ok().json(json!({"token":token}))
                        },
                        Err(e) => {
                            // Failed to sign JWT token, return a 500 Internal Server Error
                            HttpResponse::InternalServerError().json(json!({"server_error":e.to_string()}))
                        }
                    }
                } else {
                    // Passwords don't match, return a 401 Unauthorized error as a JSON response
                    HttpResponse::Unauthorized().json(json!({"message":"Invalid password"}))
                }
            } else {
                // User not found, return a 404 Not Found error as a JSON response
                HttpResponse::NotFound().json(json!({"message":"User not found"}))
            }
        }
        Err(error) => {
            // Return an error message as a JSON response
            HttpResponse::InternalServerError().json(json!({"message":format!("Failed to fetch user: {}", error)}))
        }
    }
}

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/user/create")
            .route(web::post().to(create_user))
    )
    .service(
        web::resource("/user/fetch/{id}")
            .route(web::get().to(fetch_user_by_id))
    )
    .service(
        web::resource("/user/login")
            .route(web::post().to(login))
    )
    .service(
        web::resource("/user/changeavatar/{id}")
            .route(web::post().to(upload_avatar))
    );
}
