use actix_web::{Error, HttpRequest, HttpResponse, web};
use actix_web_actors::ws;
use deadpool_lapin::lapin::options::QueueDeclareOptions;
use deadpool_lapin::lapin::types::FieldTable;
use deadpool_lapin::Pool;
use crate::common;

use crate::common::encryption;
use crate::server::ws_server;

/// do websocket handshake and start `MyWebSocket` actor
pub async fn handle(pool: web::Data<Pool>,
                  message_encryptor: web::Data<encryption::MessageEncryptor>,
                  request: HttpRequest,
                  paths: web::Path<String>,
                  stream: web::Payload) -> Result<HttpResponse, Error> {
    log::debug!("{:?}", request);
    let client_public_key = request.headers().get(common::HEADER_PUBLIC_KEY).unwrap().to_str().unwrap();

    let credentials = request.headers().get(common::HEADER_PASSWORD).and_then(|password_value| {
        let username = request.headers().get(common::HEADER_USERNAME).unwrap().to_str().unwrap().to_string();
        let password = password_value.to_str().unwrap().to_string();
        // We use the same format as Basic Auth
        Some(format!("{}:{}", username, password))
    });

    let client_id = paths.into_inner();
    let connection = pool.get().await.unwrap();
    let channel = connection.create_channel().await.unwrap();
    let queue_req_name = format!("{}_req", client_id);
    let queue_res_name = format!("{}_res", client_id);

    log::info!("Received connection request from client ID: {}", client_id);
    log::debug!("Client Public Key: {:?}", client_public_key);

    let _ = channel.queue_declare(
        &queue_req_name,
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await;
    let _ = channel.queue_declare(
        &queue_res_name,
        QueueDeclareOptions::default(),
        FieldTable::default(),
    ).await;
    let actor = ws_server::MyWebSocket::new(
        client_id,
        credentials,
        message_encryptor.get_ref().clone(),
        client_public_key.to_string(),
        channel,
        queue_req_name,
        queue_res_name
    );
    let response = ws::start(actor, &request, stream);
    log::debug!("{:?}", response);
    response
}
