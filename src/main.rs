use dotenv::dotenv;
use hyper::body::Incoming;
use hyper::header::HOST;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::env;
use std::net::SocketAddr;
use std::num::ParseIntError;
use std::sync::LazyLock;
use tokio::net::TcpListener;

#[derive(Debug)]
#[expect(dead_code, reason = "used for debugging")]
struct Error {
    error: Box<dyn std::error::Error + Send + Sync>,
    message: String,
}

impl Error {
    fn from_both(message: &str, error: &str) -> Self {
        Self {
            message: message.to_owned(),
            error: error.into(),
        }
    }

    fn from_msg<E: std::error::Error + Send + Sync + 'static>(
        message: &str,
    ) -> impl FnOnce(E) -> Self {
        |error| Self {
            error: Box::new(error),
            message: message.to_owned(),
        }
    }
}

impl std::error::Error for Error {}
impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}

const DOMAIN_BODY: LazyLock<String> = LazyLock::new(|| {
    dotenv().unwrap();
    env::var("DOMAIN_BODY").unwrap()
});

fn default_middleware(_req: Request<Incoming>) -> Result<Response<String>, Error> {
    Response::builder()
        .body("Default router\n".to_owned())
        .map_err(Error::from_msg("Failed to build default response"))
}

fn api_middleware(_req: Request<Incoming>) -> Result<Response<String>, Error> {
    Response::builder()
        .body("API router\n".to_owned())
        .map_err(Error::from_msg("Failed to build default response"))
}

async fn middleware(req: Request<Incoming>) -> Result<Response<String>, Error> {
    let host = req
        .headers()
        .get(HOST)
        .ok_or_else(|| Error::from_both("Failed to extract host", "Missing HOST in headers"))?
        .to_str()
        .map_err(Error::from_msg("Invalid header value for host"))?;
    let try_subdomain = host
        .split('.')
        .next()
        .ok_or_else(|| Error::from_both("Invalid host", "Missing"))?;
    let subdomain = if try_subdomain == *DOMAIN_BODY {
        None
    } else {
        Some(try_subdomain)
    };
    match subdomain {
        None | Some("www") => default_middleware(req),
        Some("api") => api_middleware(req),
        Some(_) => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body("Page not found | Invalid subdomain.\n".to_owned())
            .map_err(Error::from_msg("Failed to build 404")),
    }
}

fn get_address() -> Result<SocketAddr, Error> {
    dotenv().map_err(Error::from_msg("Failed to load .env file"))?;
    let port = env::var("SERVER_PORT")
        .map_err(Error::from_msg("Missing SERVER_PORT in .env"))?
        .parse::<u16>()
        .map_err(Error::from_msg("SERVER_PORT is not a valid port number"))?;
    let host_vec: Vec<u8> = env::var("SERVER_HOST")
        .map_err(Error::from_msg("Missing SERVER_HOST in .env"))?
        .split(".")
        .into_iter()
        .map(|elt| elt.parse::<u8>())
        .collect::<Result<Vec<_>, ParseIntError>>()
        .map_err(Error::from_msg("SERVER_HOST is not a valid IPv4 address."))?;
    let host: [u8; 4] = host_vec.try_into().map_err(|_invalid| {
        Error::from_both(
            "SERVER_HOST isn't a valid IPv4 address.",
            "Too many numbers.",
        )
    })?;
    Ok(SocketAddr::from((host, port)))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let addr = get_address()?;
    let listener = TcpListener::bind(addr)
        .await
        .map_err(Error::from_msg("Failed to find address"))?;
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .map_err(Error::from_msg("Failed to find address"))?;
        let io = TokioIo::new(stream);
        tokio::task::spawn(async move {
            let builder = http1::Builder::new()
                .serve_connection(io, service_fn(middleware))
                .await;
            if let Err(err) = builder {
                eprintln!("Error serving connection: {err:?}");
            }
        });
    }
}
