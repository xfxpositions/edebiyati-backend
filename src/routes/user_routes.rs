use std::str::FromStr;

use actix_web::{web, HttpResponse, Responder};
use mongodb::{Database, bson::{self, doc, from_document, oid::ObjectId}, Collection, options::FindOneOptions};
use serde::Deserialize;
use serde_json::json;
use sha256::digest;

use crate::types::User;
use crate::utils::sign_jwt;

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
        Ok(_) => HttpResponse::Ok().body("User created successfully"),
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
                        Err(_) => {
                            // Failed to sign JWT token, return a 500 Internal Server Error
                            HttpResponse::InternalServerError().finish()
                        }
                    }
                } else {
                    // Passwords don't match, return a 401 Unauthorized error as a JSON response
                    HttpResponse::Unauthorized().json("Invalid password")
                }
            } else {
                // User not found, return a 404 Not Found error as a JSON response
                HttpResponse::NotFound().json("User not found")
            }
        }
        Err(error) => {
            // Return an error message as a JSON response
            HttpResponse::InternalServerError().json(format!("Failed to fetch user: {}", error))
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
    );
}