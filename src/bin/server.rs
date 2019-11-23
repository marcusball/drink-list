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
use chrono::NaiveDate;
use diesel::prelude::*;
use diesel::r2d2::ConnectionManager;
use futures::future::Either;
use futures::Future;
use regex::Regex;

use drink_list::api::{ApiResponse, ResponseStatus};
use drink_list::db;
use drink_list::db::{Connection, GetDrinks, Pool};
use drink_list::reports::{DrinkAggregate, DrinkAggregator};

#[derive(Serialize)]
struct AggregatedEntry {
    pub entry: db::Entry,
    pub aggregate: DrinkAggregate,
}

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

/// Route to get all drinks from all time.
fn get_entries(pool: web::Data<Pool>) -> impl Future<Item = HttpResponse, Error = Error> {
    get_entries_internal(pool, None)
}

fn get_entries_by_date(
    (pool, path): (web::Data<Pool>, web::Path<NaiveDate>),
) -> impl Future<Item = HttpResponse, Error = Error> {
    let date = path.into_inner();
    get_entries_internal(pool, Some((date.clone(), date)))
}

/// Internal route handler, to allow other routes to all share the same handler code.
///
fn get_entries_internal(
    pool: web::Data<Pool>,
    date_range: Option<(NaiveDate, NaiveDate)>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    #[derive(Serialize)]
    #[serde(rename = "drinks")]
    struct Drinks(Vec<AggregatedEntry>);

    db::execute(
        &pool,
        GetDrinks {
            person_id: 1,
            date_range: date_range,
        },
    )
    .from_err()
    .and_then(|res| match res {
        Ok(drinks) => {
            let drinks = Drinks(
                drinks
                    .into_iter()
                    .map(|entry| AggregatedEntry {
                        aggregate: entry.aggregate(),
                        entry: entry,
                    })
                    .collect(),
            );

            Ok(HttpResponse::Ok().json(ApiResponse::success(drinks)))
        }
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
                    .service(web::resource("").route(web::get().to_async(get_entries)))
                    .service(
                        web::resource("/{date}").route(web::get().to_async(get_entries_by_date)),
                    ),
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
