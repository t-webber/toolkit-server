use http_body_util::Full;
use hyper::body::Bytes;
use hyper::header::HOST;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use tokio::net::TcpListener;

const DOMAIN_BODY: &str = "lvh";
const SERVER_HOST: [u8; 4] = [127, 0, 0, 1];
const SERVER_PORT: u16 = 3000;

async fn middleware(req: Request<hyper::body::Incoming>) -> Result<Response<Full<Bytes>>, String> {
    let host = req
        .headers()
        .get(HOST)
        .ok_or_else(|| "Failed to extract host")?
        .to_str()
        .map_err(|err| err.to_string())?;
    let try_subdomain = host.split('.').next().ok_or_else(|| "Invalid host")?;
    let subdomain = if try_subdomain == DOMAIN_BODY {
        None
    } else {
        Some(try_subdomain)
    };
    Ok(Response::new(Full::new(Bytes::from(format!(
        "{subdomain:?}\n"
    )))))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = SocketAddr::from((SERVER_HOST, SERVER_PORT));
    let listener = TcpListener::bind(addr).await?;

    loop {
        let (stream, _) = listener.accept().await?;
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
