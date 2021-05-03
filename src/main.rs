#![feature(proc_macro_hygiene, decl_macro, try_trait)]

#[macro_use]
extern crate rocket;

use futures::try_join;
use std::io;
use tokio::task;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

mod cli;
mod dns;
mod errors;
mod http;

#[tokio::main]
async fn main() {
    let args = cli::args().expect("error parsing args");

    // create an object to wrap DNS server
    let dns_api = dns::DnsApi::new().await;

    let dns_future = task::spawn(dns_api.serve());
    let http_future = task::spawn(http::server(args.sled_data_path));

    // join futures
    let result = try_join!(dns_future, http_future);

    // insert data
    let test_ip = Ipv4Addr::new(1, 1, 1, 1);
    let domain = "test.dyn.peach.cloud.";
    dns_api.upsert(domain.to_string(), test_ip);

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
