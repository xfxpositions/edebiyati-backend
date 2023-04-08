use actix_web::{web, HttpResponse, Responder, HttpRequest, post, get};

#[get("/{id}")]
async fn create_post(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

#[post("/")]
async fn fetch_post(_req: HttpRequest) -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

pub fn post_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/post")
            .service(create_post)
            .service(fetch_post)
    );
}
