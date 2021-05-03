/*
* ROUTES
* /domain/put : create or update a domain record
* /domain/available/<domain> : check if a given domain is free
*
*/
use crate::errors::*;
use rocket::State;
use rocket_contrib::json::Json;
use serde::{Deserialize, Serialize};
use sled_extensions::bincode::Tree;
use sled_extensions::DbExt;
use snafu::ResultExt;

// this is the object type returned by rocket route handlers
type EndpointResult<T> = Result<T, ServerError>;

#[get("/")]
fn index() -> &'static str {
    "This is the peach-dyn-dns server."
}

#[get("/test_error")]
fn test_error() -> Result<&'static str, ServerError> {
    info!("++ get request to test_error");
    TestError {
        msg: "this is a test error".to_string(),
    }
    .fail()
}

// the json format used by put_domain
#[derive(Deserialize, Serialize, Clone, Debug)]
struct DomainRecord {
    domain: String,
    secret: String,
    ip: String,
}

#[post("/domain/put", data = "<data>")]
fn put_domain(db: State<Database>, data: Json<DomainRecord>) -> EndpointResult<Json<DomainRecord>> {
    // this route creates or updates a record for a given domain
    // if a record with the given domain does not already exist,
    // then it creates one, using the secret supplied in the post
    // if a record already exists
    // it checks if the secret in the post matches the secret in the record,
    // if there is a match, then it updates the record
    // if they don't match, then it returns a 403 error
    info!("post request to /domain/put: {:?}", data);
    // first check if a domain record with this domain name already exists
    let previous_domain_record = db.domains.get(data.domain.as_bytes()).context(SledError)?;
    info!("found previous_domain_record: {:?}", previous_domain_record);
    // then match based on Some Or None
    match previous_domain_record {
        // if a record already exists with this domain name
        Some(previous) => {
            // check if the secret matches up
            if data.secret == previous.secret {
                info!("secrets match");
                // if the secret matches, then update the record
                db.domains
                    .insert(data.domain.as_bytes(), data.clone())
                    .unwrap();
                Ok(Json(data.0))
            // if the secret doesn't match, then return an error
            } else {
                info!("secrets mismatch");
                return SecretMismatch {
                    domain: format!("{:?}", data.domain),
                }
                .fail();
            }
        }
        // if a previous_domain_record was not found with this domain name, then create one
        None => {
            db.domains
                .insert(data.domain.as_bytes(), data.clone())
                .unwrap();
            Ok(Json(data.0))
        }
    }
}

// the json format returned by is_domain_available
#[derive(Deserialize, Serialize, Clone, Debug)]
struct IsDomainAvailableResponse {
    domain: String,
    available: bool,
}

#[get("/domain/available/<domain>")]
fn is_domain_available(
    db: State<Database>,
    domain: String,
) -> EndpointResult<Json<IsDomainAvailableResponse>> {
    // returns true if the given domain is available
    // false if an an already existant record is found
    info!("get request to /domain/available/{:?}", domain);
    // check if a domain record with this domain name already exists
    let previous_domain_record = db.domains.get(domain.as_bytes()).context(SledError)?;
    // then match based on Some Or None
    match previous_domain_record {
        // if a record already exists with this domain name
        Some(_previous) => Ok(Json(IsDomainAvailableResponse {
            domain,
            available: false,
        })),
        // if a record does not exist
        None => Ok(Json(IsDomainAvailableResponse {
            domain,
            available: true,
        })),
    }
}

// create a tree for each data type in sled
// from https://mbuffett.com/posts/rocket-sled-tutorial/
struct Database {
    domains: Tree<DomainRecord>,
}

// create a rocket server
pub async fn server(sled_data_path: String) {
    info!("using sled data path: {:?}", sled_data_path);
    let db = sled_extensions::Config::default()
        .path(sled_data_path)
        .open()
        .expect("Failed to open sled db");

    let rocket_result = rocket::build()
        .manage(Database {
            domains: db
                .open_bincode_tree("domains")
                .expect("could not create sled tree"),
        })
        .mount(
            "/",
            routes![index, put_domain, test_error, is_domain_available],
        )
        .launch()
        .await;

    if let Err(err) = rocket_result {
        error!("error launching rocket server: {:?}", err);
    }
}
