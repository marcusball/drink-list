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
use drink_list::db::{Connection, CreateDrink, CreateEntry, GetDrink, GetDrinks, GetEntry, Pool};
use drink_list::import::{Abv, QuantityRange, VolumeContext};
use drink_list::models::TimePeriod;
use drink_list::reports::{DrinkAggregate, DrinkAggregator};

#[derive(Serialize)]
#[serde(rename = "aggregated_entry")]
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

#[derive(Deserialize)]
struct EntryForm {
    pub drank_on: NaiveDate,

    pub time_period: String,

    pub quantity: String,

    pub name: String,

    pub abv: Option<String>,

    pub volume: Option<String>,
}

fn new_entry(
    pool: web::Data<Pool>,
    form: web::Form<EntryForm>,
) -> impl Future<Item = HttpResponse, Error = Error> {
    use futures::future;

    let time_period = match TimePeriod::from_str(&form.time_period.to_lowercase()) {
        Some(time_period) => time_period,
        None => {
            info!(
                "Received invalid time period input, '{}'!",
                form.time_period
            );
            let response = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Fail)
                .add_message("Invalid time period value!".into());
            return Either::A(futures::future::ok(
                HttpResponse::BadRequest().json(response),
            ));
        }
    };
    // Attempt to parse the quantity string.
    let quantity = match QuantityRange::from_str(&form.quantity) {
        Ok(quantity) => quantity,
        Err(e) => {
            info!("Received invalid quantity input, '{}'!", form.quantity);
            let response = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Fail)
                .add_message("Invalid quantity value!".into());
            return Either::A(futures::future::ok(
                HttpResponse::BadRequest().json(response),
            ));
        }
    };

    // Now attempt to parse the ABV string.
    let abv = match form.abv.as_ref().map(Abv::from_str).transpose() {
        Ok(abv) => abv.flatten(),
        Err(e) => {
            info!(
                "Received invalid ABV input, '{}'!",
                form.abv.as_ref().unwrap()
            );
            let response = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Fail)
                .add_message("Invalid ABV value!".into());
            return Either::A(futures::future::ok(
                HttpResponse::BadRequest().json(response),
            ));
        }
    };

    // Parse the volume string.
    let volume = match form
        .volume
        .as_ref()
        .map(VolumeContext::from_str)
        .transpose()
    {
        Ok(volume) => volume.flatten(),
        Err(e) => {
            info!(
                "Received invalid Volume input, '{}'!",
                form.volume.as_ref().unwrap()
            );
            let response = ApiResponse::<()>::from(None)
                .with_status(ResponseStatus::Fail)
                .add_message("Invalid Volume value!".into());
            return Either::A(futures::future::ok(
                HttpResponse::BadRequest().json(response),
            ));
        }
    };

    // Finally, normalize the name
    let name = form.name.trim();

    // And attempt to derive a multiplier, if needed.
    let multiplier = match name.to_lowercase().contains("double") {
        true => 2.0,
        false => 1.0,
    };

    /*********************************************/
    /*  Closures for database operations         */
    /*********************************************/

    // Create a new drink record.
    let create_drink = |pool: &Pool, name: String, abv: Option<Abv>, multiplier: f32| {
        db::execute(
            pool,
            CreateDrink {
                name,
                abv,
                multiplier,
            },
        )
        .from_err()
        .and_then(|res| res)
        .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will attempt to get an existing drink record.
    // If none is found, it will create a new drink record.
    let get_or_create_drink = |pool: &Pool, name: String, abv: Option<Abv>, multiplier: f32| {
        let pool_clone = pool.clone();
        db::execute(
            &pool,
            GetDrink {
                name: name.clone(),
                abv: abv.clone(),
            },
        )
        .from_err()
        .and_then(move |res| match res {
            Ok(Some(drink)) => Either::A(future::result(Ok(drink))),
            Ok(None) => Either::B(create_drink(&pool_clone, name, abv, multiplier)),
            Err(e) => Either::A(future::result(Err(actix_web::Error::from(e)))),
        })
    };

    // This closure will create a new entry record.
    let create_entry = |pool: &Pool,
                        person_id: i32,
                        drank_on: NaiveDate,
                        time_period: TimePeriod,
                        context: Vec<String>,
                        drink_id: i32,
                        quantity: QuantityRange,
                        volume: Option<VolumeContext>| {
        db::execute(
            &pool,
            CreateEntry {
                person_id,
                drank_on,
                time_period,
                context,
                drink_id,
                quantity,
                volume,
            },
        )
        .from_err()
        .and_then(|res| res)
        .map_err(|e| actix_web::Error::from(e))
    };

    // This closure will lookup the full details of the given entry.
    let get_entry = |pool: &Pool, person_id: i32, entry_id: i32| {
        db::execute(
            &pool,
            GetEntry {
                person_id,
                entry_id,
            },
        )
        .from_err()
        .and_then(|res| res)
        .map_err(|e| actix_web::Error::from(e))
    };

    /*********************************************/
    /* Begin actual function execution           */
    /*********************************************/

    let pool_clone = pool.clone();

    Either::B(
        // Lookup the drink details if a record exists, otherwise create a new record.
        get_or_create_drink(&pool, name.to_string(), abv, multiplier)
            // Now create a new entry using the drink details.
            .and_then(move |drink| {
                create_entry(
                    &pool,
                    1,
                    form.drank_on,
                    time_period,
                    Vec::new(),
                    drink.id,
                    quantity,
                    volume,
                )
            })
            // Lookup the full details of the entry we just created.
            .and_then(move |entry| get_entry(&pool_clone, 1, entry.id))
            // Generate output
            .then(|res| match res {
                // All good, return the entry.
                Ok(Some(entry)) => {
                    let output = AggregatedEntry {
                        aggregate: entry.aggregate(),
                        entry: entry,
                    };

                    Ok(HttpResponse::Ok().json(ApiResponse::success(output)))
                }
                // This case should be impossible; it would only happen if no record was found matching the entry ID.
                Ok(None) => {
                    error!("An entry was created but retrieval returned no results.");
                    Ok(HttpResponse::InternalServerError().into())
                }
                // Everything exploded.
                Err(e) => {
                    error!("An error occurred: {}", e);
                    Ok(HttpResponse::InternalServerError().into())
                }
            }),
    )
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
                    .service(
                        web::resource("")
                            .route(web::get().to_async(get_entries))
                            .route(web::post().to_async(new_entry)),
                    )
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
