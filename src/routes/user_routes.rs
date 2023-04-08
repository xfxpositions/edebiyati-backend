use actix_web::{web, HttpResponse, Responder};


async fn create_user()->impl Responder {
    HttpResponse::Ok()
}

async fn fetch_user()->impl Responder {
    HttpResponse::Ok()
}

pub fn user_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::resource("/user/create")
            .route(web::post().to(create_user))
    )
    .service(
        web::resource("/user/fetch/{id}")
            .route(web::get().to(fetch_user))
    );
}