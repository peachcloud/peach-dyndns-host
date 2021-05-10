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


struct DnsManager {
    catalog: Catalog,
}

impl DnsManager {
    pub fn new() -> DnsManager {

        let catalog: Catalog = Catalog::new();

        return DnsManager {
            catalog,
        };
    }

    pub fn upsert(&mut self, domain: String, ip: Ipv4Addr) {


        let authority_name = Name::from_str("dyn.peachcloud.org.").unwrap();

        let soa_serial = 1;
        let soa_name = Name::from_str("dyn.peachcloud.org.").unwrap();
        let soa_rdata = RData::SOA(SOA::new(
            Name::from_str("dyn.peachcloud.org.").unwrap(),      // mname
            Name::from_str("root.dyn.peachcloud.org.").unwrap(), // rname
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

        let authority_zone_type = ZoneType::Master;
        let authority_allow_axfr = false;

        let mut authority = InMemoryAuthority::new(
            authority_name.clone(),
            authority_records,
            authority_zone_type,
            authority_allow_axfr,
        )
            .unwrap();

            /*
        let ns_name = Name::from_str("dyn.peachcloud.org.").unwrap();
        let ns_ttl = 60;
        let ns_rdata = RData::NS(Name::from_str("localhost.").unwrap());
        let ns_record = Record::from_rdata(ns_name, ns_ttl, ns_rdata);
        authority.upsert(ns_record, authority.serial());
        */

        let dyn_name = Name::from_str(&domain).unwrap();
        let dyn_ttl = 60;
        let dyn_rdata = RData::A(ip);
        let dyn_record = Record::from_rdata(dyn_name, dyn_ttl, dyn_rdata);
        authority.upsert(dyn_record, authority.serial());

        self.catalog.upsert(
            LowerName::new(&authority_name),
            Box::new(Arc::new(RwLock::new(authority))),
        );
    }

    fn get_initial_records(&mut self) ->  BTreeMap<RrKey, RecordSet> {
        let authority_name = Name::from_str("dyn.peachcloud.org.").unwrap();
        let soa_serial = 1;
        let soa_name = Name::from_str("dyn.peachcloud.org.").unwrap();
        let soa_rdata = RData::SOA(SOA::new(
            Name::from_str("dyn.peachcloud.org.").unwrap(),      // mname
            Name::from_str("root.dyn.peachcloud.org.").unwrap(), // rname
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

    fn upsert_test(&mut self) {

        let authority_records = self.get_initial_records();

        let authority_name = Name::from_str("dyn.peachcloud.org.").unwrap();

        let authority_zone_type = ZoneType::Master;
        let authority_allow_axfr = false;

        // first upsert
        let domain1 = "test.dyn.peachcloud.org";
        let ip1 = Ipv4Addr::new(1, 1, 1, 1);

        let mut authority = InMemoryAuthority::new(
            authority_name.clone(),
            authority_records,
            authority_zone_type,
            authority_allow_axfr,
        )
            .unwrap();

        let dyn_name = Name::from_str(domain1).unwrap();
        let dyn_ttl = 60;
        let dyn_rdata = RData::A(ip1);
        let dyn_record = Record::from_rdata(dyn_name, dyn_ttl, dyn_rdata);
        authority.upsert(dyn_record, authority.serial());

        let authority_ref = &authority;

        self.catalog.upsert(
            LowerName::new(&authority_name),
            Box::new(Arc::new(RwLock::new(authority))),
        );

        // test second insert
        let domain2 = "who.dyn.peachcloud.org";
        let ip2 = Ipv4Addr::new(1, 1, 1, 2);
        let dyn_name = Name::from_str(domain2).unwrap();
        let dyn_ttl = 60;
        let dyn_rdata = RData::A(ip2);
        let dyn_record = Record::from_rdata(dyn_name, dyn_ttl, dyn_rdata);
        authority.upsert(dyn_record, authority.serial());

        // test if it worked
        let dyn_name = Name::from_str(domain1).unwrap();
        let lower_name = LowerName::from(dyn_name);
        let found = self.catalog.contains(&lower_name);
        println!("++ found {:?}: {:?}", lower_name.to_string(), found);

        let dyn_name = Name::from_str(domain2).unwrap();
        let lower_name = LowerName::from(dyn_name);
        let found = self.catalog.contains(&lower_name);
        println!("++ found {:?}: {:?}", lower_name.to_string(), found);

        let lower_name = LowerName::new(&authority_name);
        let found = self.catalog.contains(&lower_name);
        println!("++ found {:?}: {:?}", lower_name.to_string(), found);


//        // second upsert
//        let domain2 = "peach.dyn.peachcloud.org";
//        let ip2 = Ipv4Addr::new(1, 1, 1, 2);
//        let authority_records = self.get_initial_records();
//
//        let mut authority = InMemoryAuthority::new(
//            authority_name.clone(),
//            authority_records,
//            authority_zone_type,
//            authority_allow_axfr,
//        )
//            .unwrap();
//
//        let dyn_name = Name::from_str(domain2).unwrap();
//        let dyn_ttl = 60;
//        let dyn_rdata = RData::A(ip2);
//        let dyn_record = Record::from_rdata(dyn_name, dyn_ttl, dyn_rdata);
//        authority.upsert(dyn_record, authority.serial());
//
//        self.catalog.upsert(
//            LowerName::new(&authority_name),
//            Box::new(Arc::new(RwLock::new(authority))),
//        );
    }
}


pub async fn server() -> ServerFuture<Catalog> {
    info!("Trust-DNS {} starting", trust_dns_server::version());

    let mut dns_manager = DnsManager::new();

//    // first insert
//    dns_manager.upsert(
//        "test.dyn.peachcloud.org".to_string(),
//        Ipv4Addr::new(1, 1, 1, 1),
//    );
//
//    // second insert
//    dns_manager.upsert(
//        "test.dyn.peachcloud.org".to_string(),
//        Ipv4Addr::new(1, 1, 1, 3),
//    );
//
//    // third insert
//    dns_manager.upsert(
//        "peach.dyn.peachcloud.org".to_string(),
//        Ipv4Addr::new(1, 1 , 2, 3),
//    );

    dns_manager.upsert_test();

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

    let mut server = ServerFuture::new(dns_manager.catalog);

    // load all the listeners
    info!("DNS server listening for UDP on {:?}", udp_socket);
    server.register_socket(udp_socket);

    info!("DNS server listening for TCP on {:?}", tcp_listener);
    server.register_listener(tcp_listener, tcp_request_timeout);
    info!("awaiting DNS connections...");

    server
}
