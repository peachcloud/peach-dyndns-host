#![feature(proc_macro_hygiene, decl_macro, try_trait)]

#[macro_use]
extern crate rocket;

use futures::try_join;
use std::io;
use tokio::task;


mod cli;
mod dns;
mod errors;
mod http;

#[tokio::main]
async fn main() {
    let args = cli::args().expect("error parsing args");

    // create future for dns and http servers
    let dns_future = task::spawn(dns::server());
    let http_future = task::spawn(http::server(args.sled_data_path));

    // join futures
    let result = try_join!(dns_future, http_future);

    match result {
        Err(e) => {
            io::Error::new(
                io::ErrorKind::Interrupted,
                "Server stopping due to interruption",
            );
            error!("server failure: {}", e);
        }
        Ok(_val) => {
            info!("we're stopping for some unexpected reason");
        }
    }
    info!("we're stopping for some unexpected reason");
}
