use std::{str::FromStr, io::Write, collections::HashMap, env};

use actix_web::{web::{self}, HttpResponse, Responder, dev::{ServiceRequest, Service}, Resource, http::Error};
use actix_multipart::Multipart;
use reqwest::{Client, Url};

use mongodb::{Database, bson::{self, doc, from_document, oid::ObjectId}, options::{FindOneAndUpdateOptions, ReturnDocument}, Collection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha256::digest;
use dotenv::dotenv;

use crate::{types::User, utils::upload_image_to_s3};
use crate::utils::sign_jwt;
use futures::{StreamExt, TryStreamExt, FutureExt};
use uuid::Uuid;
async fn print_headers_middleware<AppState>(
    req: ServiceRequest,
    srv: &Resource<AppState>,
) -> Result<ServiceRequest, actix_web::Error> {
    // Print the request headers
    println!("Headers: {:?}", req.headers());

    // Pass the request to the next middleware or handler
    Ok(req)
}



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

async fn update_user(
    user_id: web::Path<String>,
    new_data: web::Json<HashMap<String, String>>,
    db: web::Data<Database>
) -> impl Responder {
    let collection = db.collection::<User>("users");

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

    let mut update_fields = doc! {};

    for (key, value) in new_data.iter() {
        update_fields.insert(key, value);
    }

    let update_doc = doc! {"$set": update_fields};

    let result = collection
        .update_one(doc! {"_id": id}, update_doc, None)
        .await;

    match result {
        Ok(user) => HttpResponse::Ok().json(json!({"user": user})),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error creating user: {}", e))
    }
}



#[derive(Deserialize)]
struct UpdatePasswordRequest {
    old_password: String,
    new_password: String,
}

async fn update_password(
    user_id: web::Path<String>,
    password_data: web::Json<UpdatePasswordRequest>,
    db: web::Data<Database>
) -> impl Responder {
    let collection = db.collection::<User>("users");

    fn is_valid_objectid(id: &str) -> bool {
        if let Ok(_) = ObjectId::from_str(id) {
            true
        } else {
            false
        }
    }

    if !is_valid_objectid(&user_id) {
        return HttpResponse::BadRequest().json(json!({"error": "Invalid user ID"}));
    }

    fn hash_password(password: String) -> String {
        let hashed_password = digest(password);
        hashed_password
    }

    let id = ObjectId::from_str(&user_id).unwrap();

    match collection.find_one(doc! {"_id": id}, None).await {
        Ok(result) => {
            if let Some(user) = result {
                if user.password == hash_password(password_data.old_password.clone()){
                    let new_password_encrypted = hash_password(password_data.new_password.clone());
                
                    let update_doc = doc! {"$set": {"password" : new_password_encrypted }};
                
                    let result = collection
                        .update_one(doc! {"_id": id}, update_doc, None)
                        .await;
        
                    match result {
                        Ok(user) => HttpResponse::Ok().json(json!({"success": "Password changed successfully"})),
                        Err(e) => HttpResponse::InternalServerError().json(json!({"error": format!("Error changing password: {}", e)}))
                    }
                } else {
                    HttpResponse::Unauthorized().json(json!({"error": "Incorrect password"}))
                }
            
            } else {
                HttpResponse::NotFound().json(json!({"error": "User not found"}))
            }
        }
        Err(error) => {
            HttpResponse::InternalServerError().json(json!({"error": format!("Failed to fetch user: {}", error)}))
        }
    } 
}


#[derive(Deserialize)]
struct GetGoogleUserRequest {
    token:String
}

async fn get_google_user(
    token_data: web::Json<GetGoogleUserRequest>
) -> impl Responder {
    dotenv().ok();
    async fn request_token(token_data:String) -> Result<String, Box<dyn std::error::Error>> {
        let redirect_url = env::var("GOOGLE_REDIRECT_URL")?;
        let client_secret = env::var("GOOGLE_CLIENT_SECRET")?;
        let client_id = env::var("GOOGLE_CLIENT_ID")?;

        let root_url = "https://oauth2.googleapis.com/token";
        let client = Client::new();

        let params = [
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_url.as_str()),
            ("client_id", client_id.as_str()),
            ("code", token_data.as_str()),
            ("client_secret", client_secret.as_str()),
        ];
        let response = client.post(root_url).form(&params).send().await?;

        #[derive(Debug,Serialize,Deserialize)]
        struct Token{
            access_token:String,
            expires_in:u32,
            refresh_token:String,
            scope:String,
            token_type:String,
            id_token:String
        }

        if response.status().is_success() {
            let oauth_response = response.text().await?;
            let token:Token = serde_json::from_str(oauth_response.as_str())?;
            println!("{}",oauth_response);
            Ok(token.access_token)
        } else {
            let oauth_response = response.text().await?;
            println!("BURADA HATA VAR ALOOO{}",oauth_response);
            Err(format!("Failed to get token: {}", oauth_response).into())
        }
    }

    let access_token = match request_token(token_data.token.clone()).await {
        Ok(token) => token,
        Err(err) => {
            println!("Error: {}", err);
            return HttpResponse::Unauthorized().json(json!({"error": err.to_string()}));
        }
    };

    let client = Client::new();
    let mut url = Url::parse("https://www.googleapis.com/oauth2/v1/userinfo").unwrap();
    url.query_pairs_mut().append_pair("alt", "json");
    url.query_pairs_mut()
        .append_pair("access_token", access_token.clone().as_str());

    let response = match client.get(url).bearer_auth(access_token.clone()).send().await {
        Ok(response) => response,
        Err(err) => {
            println!("Error: {}", err);
            return HttpResponse::Unauthorized().json(json!({"error": err.to_string()}));
        }
    };

    match response.text().await {
        Ok(usertext) => {
            println!("response {}", usertext.clone().to_string());
            #[derive(Debug, Serialize, Deserialize)]
            struct UserData {
                id: String,
                email: String,
                verified_email: bool,
                name: String,
                given_name: String,
                picture: String,
                locale: String
            }
            let user: UserData = serde_json::from_str(&usertext).unwrap();
            HttpResponse::Ok().json(user)
        },
        Err(err) => {
            println!("Error: {}", err);
            HttpResponse::Unauthorized().json(json!({"error": err.to_string()}))
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
    )
    .service(
        web::resource("/user/update/{id}")
            .route(web::post().to(update_user))
    )
    .service(
        web::resource("/user/updatepassword/{id}")
            .route(web::post().to(update_password))
    )
    .service(
        web::resource("/user/getgoogleuser")
            .route(web::post().to(get_google_user))
    );
}
