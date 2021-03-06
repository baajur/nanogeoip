#![warn(missing_docs)]
//! nanogeoip HTTP handlers and associated functionality.
use super::db::{Reader, Record};

use hyper::header::{ACCESS_CONTROL_ALLOW_ORIGIN, CONTENT_TYPE, LAST_MODIFIED};
use hyper::{Body, Request, Response, StatusCode};

use std::net::IpAddr;
use std::str::FromStr;

/// Various options modifying how the lookup service responds to HTTP requests.
pub struct Options {
    /// `Access-Control-Allow-Origin` header, for controlling CORS.
    pub cors_header: Option<String>,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            cors_header: Some("*".to_string()),
        }
    }
}

fn err_json(msg: &str) -> Body {
    Body::from(format!("{{\"error\": \"{}\"}}", msg))
}

/// Transforms a HTTP request into a response.
///
/// Essentially a Hyper "service" but not encapsulated into the Service trait just
/// yet, because type-checker Reasons™. :-(
pub fn lookup(req: Request<Body>, db: &Reader, opts: &Options) -> Response<Body> {
    let mut response = Response::builder();
    response.header(CONTENT_TYPE, "application/json");

    if let Some(ref rule) = opts.cors_header {
        response.header(ACCESS_CONTROL_ALLOW_ORIGIN, rule.to_owned());
    }

    let path = req.uri().path().trim_start_matches("/");
    if path == "" {
        return response
            .status(StatusCode::BAD_REQUEST)
            .body(err_json("missing IP query in path, try /192.168.1.1"))
            .unwrap();
    }

    let ip: IpAddr = match FromStr::from_str(path) {
        Ok(ip) => ip,
        Err(_e) => {
            return response
                .status(StatusCode::BAD_REQUEST)
                .body(err_json("could not parse invalid IP address"))
                .unwrap();
        }
    };

    let results: Record = match db.lookup(ip) {
        Ok(res) => res,
        Err(_e) => {
            // native MaxMindDBError(String) is "error while decoding value"
            return response
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(err_json("IP not found"))
                .unwrap();
        }
    };

    let payload = serde_json::to_vec(&results).unwrap();
    response.header(LAST_MODIFIED, db.load_time_str());
    response.body(Body::from(payload)).unwrap()
}
