mod tcp;
mod ws;
use ws::WsConn;

use actix_web_actors::ws::start;
use actix_web::web::Data;
use actix_web::{get, web, App, Error, HttpRequest, HttpResponse, HttpServer, Responder};

use std::collections::HashMap;
use std::env;
use std::process::exit;
use std::sync::{Arc, Mutex};

use models::TunnelType;

pub struct AppState {
    ports: Arc<Mutex<Vec<u16>>>,
}

#[actix::main]
async fn main() {
    let public_host = env::var("PUBLIC_HOST");
    if public_host.is_err() {
        println!("environment variable PUBLIC_HOST is missing");
        exit(1);
    }

    let public_host = public_host.unwrap();

    let port = env::var("PORT");
    if port.is_err() {
        println!("environment variable PORT is missing");
        exit(1);
    }

    let port = port.unwrap().parse::<u16>();
    if port.is_err() {
        println!("PORT is not an u16 number");
        exit(1);
    }

    let port = port.unwrap();

    let tunnel_port_range = env::var("TUNNEL_PORT_RANGE");
    if tunnel_port_range.is_err() {
        println!("environment variable TUNNEL_PORT_RANGE is missing");
        exit(1);
    }

    let tunnel_port_range = tunnel_port_range.unwrap();
    let tunnel_port_range = tunnel_port_range
        .split("-")
        .filter(|s| !s.is_empty())
        .map(|p| {
            p.parse::<u16>().unwrap_or_else(|_| {
                println!("Unable to parse TUNNEL_PORT_RANGE");
                exit(1)
            })
        })
        .collect::<Vec<_>>();

    if tunnel_port_range.len() != 2 {
        println!("Unable to parse TUNNEL_PORT_RANGE");
        exit(1);
    }

    let ports = (tunnel_port_range[0]..tunnel_port_range[1] + 1).collect::<Vec<_>>();
    if ports.len() == 0 {
        println!("Zero available tunnel ports");
        exit(1);
    }

    println!("Listening on http://0.0.0.0:{}", port);
    println!("Public host on http://{}:{}", public_host, port);
    println!("Available tunnel ports {}", ports.len());

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(AppState {
                ports: Arc::new(Mutex::new(ports.clone())),
            }))
            .service(index)
            .route("/tunnel/tcp", web::get().to(ws_tcp))
    })
    .bind(("0.0.0.0", port))
    .unwrap_or_else(|e| {
        println!("error: {}", e.to_string());
        exit(1)
    })
    .run()
    .await
    .unwrap()
}

#[get("/")]
async fn index() -> impl Responder {
    "Hello, World!"
}

pub async fn ws_tcp(req: HttpRequest, stream: web::Payload, app_state: web::Data<AppState>) -> Result<HttpResponse, Error> {
    let connections = Arc::new(Mutex::new(HashMap::new()));
    let resp = start(WsConn { tunnel_type: TunnelType::TCP, app_state, connections }, &req, stream); 
    resp
}
