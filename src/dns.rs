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

static DEFAULT_TCP_REQUEST_TIMEOUT: u64 = 5;

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

pub async fn watch_sled() {

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
//                task::spawn(dns_server());
            },
            Event::Remove(key) => {}
        }
    }
}


struct DnsManager {
    catalog: Catalog,
    dyn_root_zone: String,
}

impl DnsManager {
    pub fn new(dyn_root_zone: String) -> DnsManager {

        let catalog: Catalog = Catalog::new();

        return DnsManager {
            catalog,
            dyn_root_zone,
        };
    }

    fn build_catalog(&mut self) {
        let db = sled_extensions::Config::default()
            .path("./sled_data")
            .open()
            .expect("Failed to open sled db");

        let db = Database {
            domains: db
                .open_bincode_tree("domains")
                .expect("could not create sled tree"),
        };

        self.root_upsert();

        let iter = db.domains.scan_prefix(vec![]);
        for item in iter {
            match item {
                Ok((_, DomainRecord{domain, secret, ip})) => {
                    println!("domain: {:?}", domain);
                    println!("ip: {:?}", ip);
                    let ip_result: Result<Ipv4Addr, _> = ip.parse();
                    match ip_result {
                        Ok(ip_v4) => {
                              self.upsert(&domain, ip_v4);
                        }
                        Err(e) => {
                            println!("++ found sled db entry with invalid IP format: {:?}", ip);
                        }
                    }
                }
                _ => println!("no match")
            }
        }
    }

    fn get_initial_records(domain: &str) ->  BTreeMap<RrKey, RecordSet> {
        let authority_name = Name::from_str(domain).unwrap();
        let soa_serial = 1;
        let soa_name = Name::from_str(domain).unwrap();
        let soa_rdata = RData::SOA(SOA::new(
            Name::from_str(domain).unwrap(),      // mname
            Name::from_str(&format!("root.{}", domain)).unwrap(), // rname
            soa_serial,                                       // serial
            604800,                                           // refresh
            86400,                                            // retry
            2419200,                                          // expire
            86400,                                            // negtive cache ttl
        ));
        let mut soa_record_set = RecordSet::new(&soa_name, RecordType::SOA, soa_serial);
        soa_record_set.add_rdata(soa_rdata);
        let soa_rr_key = RrKey::new(
            LowerName::new(&authority_name),
            soa_record_set.record_type(),
        );
        let mut authority_records = BTreeMap::new();
        authority_records.insert(soa_rr_key, soa_record_set);
        authority_records
    }

    pub fn upsert(&mut self, domain: &str, ip: Ipv4Addr) {

        let authority_records = DnsManager::get_initial_records(domain);
        let authority_name = Name::from_str(domain).unwrap();

        let authority_zone_type = ZoneType::Master;
        let authority_allow_axfr = false;

        let mut authority = InMemoryAuthority::new(
            authority_name.clone(),
            authority_records,
            authority_zone_type,
            authority_allow_axfr,
        )
            .unwrap();

        let dyn_name = Name::from_str(domain).unwrap();
        let dyn_ttl = 60;
        let dyn_rdata = RData::A(ip);
        let dyn_record = Record::from_rdata(dyn_name, dyn_ttl, dyn_rdata);
        authority.upsert(dyn_record, authority.serial());

        self.catalog.upsert(
            LowerName::new(&authority_name),
            Box::new(Arc::new(RwLock::new(authority))),
        );
    }

    fn root_upsert(&mut self) {
        let authority_records = DnsManager::get_initial_records(&self.dyn_root_zone);
        let authority_name = Name::from_str(&self.dyn_root_zone).unwrap();

        let authority_zone_type = ZoneType::Master;
        let authority_allow_axfr = false;

        // first upsert, for root
        let authority = InMemoryAuthority::new(
            authority_name.clone(),
            authority_records,
            authority_zone_type,
            authority_allow_axfr,
        )
            .unwrap();
        self.catalog.upsert(
            LowerName::new(&authority_name),
            Box::new(Arc::new(RwLock::new(authority))),
        );
    }

    pub async fn server(mut self) {
        let ip_addr = IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0));
            let listen_port: u16 = 12323;
            let tcp_request_timeout = Duration::from_secs(DEFAULT_TCP_REQUEST_TIMEOUT);

            let sock_addr = SocketAddr::new(ip_addr, listen_port);
            let udp_socket = UdpSocket::bind(&sock_addr)
                .await
                .expect("could not bind udp socket");
            let tcp_listener = TcpListener::bind(&sock_addr)
                .await
                .expect("could not bind tcp listener");

        let mut server = ServerFuture::new(self.catalog);

        // load all the listeners
        println!("DNS server listening for UDP on {:?}", udp_socket);
        server.register_socket(udp_socket);

        println!("DNS server listening for TCP on {:?}", tcp_listener);
        server.register_listener(tcp_listener, tcp_request_timeout);
        println!("awaiting DNS connections...");

        server.block_until_done().await;
    }
}

pub async fn dns_server() {

    dotenv::from_path("/etc/peach-dyndns.conf").ok();
    let dyn_root_zone = env::var("DYN_ROOT_ZONE").expect("DYN_ROOT_ZONE not set");

    let mut dns_manager = DnsManager::new(dyn_root_zone.to_string());
    dns_manager.build_catalog();
    dns_manager.server().await;
}


#[tokio::main]
pub async fn main() {

    watch_sled().await;

}
