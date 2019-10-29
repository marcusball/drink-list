use diesel;
use diesel::prelude::*;
use diesel::r2d2;

pub type Pool = r2d2::Pool<r2d2::ConnectionManager<PgConnection>>;
pub type Connection = r2d2::PooledConnection<r2d2::ConnectionManager<PgConnection>>;
