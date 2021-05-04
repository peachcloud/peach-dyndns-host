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

static DEFAULT_TCP_REQUEST_TIMEOUT: u64 = 5;

/// DnsApi provides an inteface for other processes to modify dns records
pub struct DnsApi {
    pub catalog: Catalog,
    pub authority: InMemoryAuthority,
    pub authority_name: trust_dns_client::rr::Name,
}

impl DnsApi {
    pub async fn new() -> DnsApi {

        let mut catalog: Catalog = Catalog::new();

        let authority_name = Name::from_str("dyn.peach.cloud.").unwrap();
        let mut authority_records = BTreeMap::new();
        let authority_zone_type = ZoneType::Master;
        let authority_allow_axfr = false;

        let soa_serial = 1;
        let soa_name = Name::from_str("dyn.peach.cloud.").unwrap();
        let soa_rdata = RData::SOA(SOA::new(
            Name::from_str("dyn.peach.cloud.").unwrap(),      // mname
            Name::from_str("root.dyn.peach.cloud.").unwrap(), // rname
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
        authority_records.insert(soa_rr_key, soa_record_set);

        let mut authority = InMemoryAuthority::new(
            authority_name.clone(),
            authority_records,
            authority_zone_type,
            authority_allow_axfr,
        )
            .unwrap();

        /*
        let ns_name = Name::from_str("dyn.peach.cloud.").unwrap();
        let ns_ttl = 60;
        let ns_rdata = RData::NS(Name::from_str("localhost.").unwrap());
        let ns_record = Record::from_rdata(ns_name, ns_ttl, ns_rdata);
        authority.upsert(ns_record, authority.serial());
        */


        let dns_api = DnsApi {
            catalog,
            authority: authority,
            authority_name: authority_name
        };
        // return the object
        dns_api
    }

    pub async fn upsert(&mut self, domain: String, ip: Ipv4Addr) {
        let dyn_name = Name::from_str(&domain).unwrap();
        let dyn_ttl = 60;
        let dyn_rdata = RData::A(ip);
        let dyn_record = Record::from_rdata(dyn_name, dyn_ttl, dyn_rdata);
        self.authority.upsert(dyn_record, self.authority.serial());

        self.catalog.upsert(
            LowerName::new(&self.authority_name),
           Box::new(Arc::new(RwLock::new(self.authority))),
        );

    }

    pub async fn serve(self) -> ServerFuture<Catalog> {

        info!("Trust-DNS {} starting", trust_dns_server::version());

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
        info!("DNS server listening for UDP on {:?}", udp_socket);
        server.register_socket(udp_socket);

        info!("DNS server listening for TCP on {:?}", tcp_listener);
        server.register_listener(tcp_listener, tcp_request_timeout);
        info!("awaiting DNS connections...");

        server
    }
}

