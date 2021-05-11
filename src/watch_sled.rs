use sled_extensions::bincode::Tree;
use sled_extensions::DbExt;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::net::TcpListener;
use tokio::net::UdpSocket;
use trust_dns_client::rr::rdata::soa::SOA;
use trust_dns_client::rr::{LowerName, Name, RData, Record, RecordSet, RecordType, RrKey};
use trust_dns_server::authority::{Catalog, ZoneType};
use trust_dns_server::server::ServerFuture;
use trust_dns_server::store::in_memory::InMemoryAuthority;
use sled::{Subscriber, Event};
use std::env;
use dotenv;
use futures::try_join;
use std::io;
use tokio::task;


// the json format used by put_domain
#[derive(Deserialize, Serialize, Clone, Debug)]
struct DomainRecord {
    domain: String,
    secret: String,
    ip: String,
}

struct Database {
    domains: Tree<DomainRecord>,
}

pub fn watch_sled() {

    dotenv::from_path("/etc/peach-dyndns.conf").ok();
    let dyn_root_zone = env::var("DYN_ROOT_ZONE").expect("DYN_ROOT_ZONE not set");

    let db = sled_extensions::Config::default()
        .path("./sled_data")
        .open()
        .expect("Failed to open sled db");

    // watch all events by subscribing to the empty prefix
    let mut subscriber = db.watch_prefix(vec![]);

    let tree_2 = db.clone();

    // `Subscription` implements `Iterator<Item=Event>`
    for event in subscriber.take(1) {
        println!("event {:?}", event);
        match event {
            Event::Insert(key, value) => {
                println!("++ sled event {:?} {:?}", key, value);

            },
            Event::Remove(key) => {}
        }
    }
}


pub fn main() {

    watch_sled();

}
