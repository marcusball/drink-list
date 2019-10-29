#[macro_use]
extern crate serde;
#[macro_use]
extern crate log;
#[macro_use]
extern crate derive_more;

use std::convert::From;
use std::str::FromStr;

use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::*;
use actix_web::{App, HttpRequest, HttpServer, Responder};
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use futures::future::Either;
use futures::Future;
use regex::Regex;

use drink_list::api::{ApiResponse, ResponseStatus};
use drink_list::db;
use drink_list::db::{Connection, GetDrinks, Pool};

fn index() -> impl Responder {
    #[derive(Serialize)]
    #[serde(rename = "message")]
    struct TestResponse(String);

    HttpResponse::Ok().json(ApiResponse::success(TestResponse("Hello world!".into())))
}

// Dummy method. Just wanted a route for the front-end to ping to make up the heroku instance.
fn wakeup() -> impl Responder {
    #[derive(Serialize)]
    #[serde(rename = "message")]
    struct TestResponse(String);

    HttpResponse::Ok().json(ApiResponse::success(TestResponse("👍".into())))
}

fn get_drinks(pool: web::Data<Pool>) -> impl Future<Item = HttpResponse, Error = Error> {
    #[derive(Serialize)]
    #[serde(rename = "drinks")]
    struct Drinks(Vec<db::Entry>);

    db::execute(&pool, GetDrinks { person_id: 1 })
        .from_err()
        .and_then(|res| match res {
            Ok(drinks) => Ok(HttpResponse::Ok().json(ApiResponse::success(Drinks(drinks)))),
            Err(_) => Ok(HttpResponse::InternalServerError().into()),
        })
}

fn main() -> std::io::Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();

    // Read the port on which to listen.
    let port = u16::from_str(&std::env::var("PORT").unwrap_or("1234".into()))
        .expect("Failed to parse $PORT!");

    // Read the IP address on which to listen
    let ip = std::net::IpAddr::from_str(&std::env::var("LISTEN_IP").unwrap_or("127.0.0.1".into()))
        .expect("Failed to parse $LISTEN_IP");

    // Construct the full Socket address
    let listen_addr = std::net::SocketAddr::new(ip, port);

    // Create a connection pool to the database
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set!");
    let manager = ConnectionManager::<PgConnection>::new(database_url);
    let pool = Pool::new(manager).expect("Failed to create database connection pool!");

    let sys = actix_rt::System::new("http-server");

    HttpServer::new(move || {
        App::new()
            .data(pool.clone())
            .wrap(Logger::default())
            .wrap(Cors::default())
            .route("/", web::get().to(index))
            .route("/wakeup", web::get().to(wakeup))
            .service(
                web::scope("/drink")
                    .service(web::resource("").route(web::get().to_async(get_drinks))),
            )

        /*.service(
            web::scope("/drink")
                .service(
                    web::resource("")
                        .route(web::get().to_async(get_drinks))
                        .route(web::post().to_async(new_drink)),
                )
                .service(web::resource("/{id}").route(web::delete().to_async(delete_drink))),
        )
        .service(
            web::scope("/auth")
                .service(web::resource("").route(web::post().to_async(begin_auth)))
                .service(web::resource("/verify").route(web::post().to_async(complete_auth)))
                .service(web::resource("/test").route(web::get().to(test_auth))),
        )
        .service(
            web::scope("/search")
                .service(web::resource("/beer").route(web::get().to_async(search_beer)))
                .service(web::resource("/brewery").route(web::get().to_async(search_brewery))),
        )*/
    })
    .bind(&listen_addr)
    .unwrap()
    .start();

    info!("Listening on {}", listen_addr);

    let _ = sys.run();

    Ok(())
}
