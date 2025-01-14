//! Actix web r2d2 example
use std::io;

use ntex::web::{self, error, middleware, App, Error, HttpResponse};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;

/// Async request handler. Ddb pool is stored in application state.
async fn index(
    path: web::types::Path<String>,
    db: web::types::State<Pool<SqliteConnectionManager>>,
) -> Result<HttpResponse, Error> {
    // execute sync code in threadpool
    let res = web::block(move || {
        let conn = db.get().unwrap();

        let uuid = format!("{}", uuid::Uuid::new_v4());
        conn.execute(
            "INSERT INTO users (id, name) VALUES ($1, $2)",
            &[&uuid, &path.into_inner()],
        )
        .unwrap();

        conn.query_row("SELECT name FROM users WHERE id=$1", &[&uuid], |row| {
            row.get::<_, String>(0)
        })
    })
    .await
    .map(|user| HttpResponse::Ok().json(&user))
    .map_err(error::ErrorInternalServerError)?;
    Ok(res)
}

#[ntex::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "ntex=debug");
    env_logger::init();

    // r2d2 pool
    let manager = SqliteConnectionManager::file("test.db");
    let pool = r2d2::Pool::new(manager).unwrap();

    // start http server
    web::server(move || {
        App::new()
            .state(pool.clone()) // <- store db pool in app state
            .wrap(middleware::Logger::default())
            .route("/{name}", web::get().to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
