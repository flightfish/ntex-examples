//! Application may have multiple data objects that are shared across
//! all handlers within same Application.
//!
//! For global shared state, we wrap our state in a `ntex::web::types::State` and move it into
//! the factory closure. The closure is called once-per-thread, and we clone our state
//! and attach to each instance of the `App` with `.app_state(state.clone())`.
//!
//! For thread-local state, we construct our state within the factory closure and attach to
//! the app with `.state(state)`.
//!
//! We retrieve our app state within our handlers with a `state: State<...>` argument.
//!
//! By default, `ntex` runs one `App` per logical cpu core.
//! When running on <N> cores, we see that the example will increment `counter1` (global state via
//! Mutex) and `counter3` (global state via Atomic variable) each time the endpoint is called,
//! but only appear to increment `counter2` every Nth time on average (thread-local state). This
//! is because the workload is being shared equally among cores.

use std::cell::Cell;
use std::io;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use ntex::web::{self, middleware, App, HttpRequest, HttpResponse};

/// simple handle
async fn index(
    counter1: web::types::State<Mutex<usize>>,
    counter2: web::types::State<Cell<u32>>,
    counter3: web::types::State<AtomicUsize>,
    req: HttpRequest,
) -> HttpResponse {
    println!("{:?}", req);

    // Increment the counters
    *counter1.lock().unwrap() += 1;
    counter2.set(counter2.get() + 1);
    counter3.fetch_add(1, Ordering::SeqCst);

    let body = format!(
        "global mutex counter: {}, local counter: {}, global atomic counter: {}",
        *counter1.lock().unwrap(),
        counter2.get(),
        counter3.load(Ordering::SeqCst),
    );
    HttpResponse::Ok().body(body)
}

#[ntex::main]
async fn main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "info");
    env_logger::init();

    // Create some global state prior to building the server
    #[allow(clippy::mutex_atomic)] // it's intentional.
    let counter1 = web::types::State::new(Mutex::new(0usize));
    let counter3 = web::types::State::new(AtomicUsize::new(0usize));

    // move is necessary to give closure below ownership of counter1
    web::server(move || {
        // Create some thread-local state
        let counter2 = Cell::new(0u32);

        App::new()
            .app_state(counter1.clone()) // add shared state
            .app_state(counter3.clone()) // add shared state
            .state(counter2) // add thread-local state
            // enable logger
            .wrap(middleware::Logger::default())
            // register simple handler
            .service(web::resource("/").to(index))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
