extern crate tiny_http;

use std::{io::{Write}, sync::{Arc, Mutex}};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
struct MessageSent {
    username: String,
    message: String,
}

fn create_header(header: &str, value: &str) -> tiny_http::Header {
    tiny_http::Header::from_bytes(header, value).unwrap()
}

/// Turns a Sec-WebSocket-Key into a Sec-WebSocket-Accept.
/// Feel free to copy-paste this function, but please use a better error handling.
fn convert_key(input: &str) -> String {
    use sha1::{Sha1, Digest};
    use base64::{engine::general_purpose, Engine as _};

    let mut hasher = Sha1::new();
    hasher.update(input.as_bytes());
    hasher.update(b"258EAFA5-E914-47DA-95CA-C5AB0DC85B11");
    let result = hasher.finalize();
    general_purpose::STANDARD.encode(result)
}

fn main() {
    let address = "192.168.15.4";
    let port = 8080;
    let server_address = format!("{}:{}", &address, &port);

    let server = tiny_http::Server::http(&server_address).unwrap();

    let messages_sent: Arc<Mutex<Vec<MessageSent>>> = Arc::new(Mutex::new(vec![]));

    println!("Server is UP on address: {}", &server_address);

    loop {
        match server.recv() {
            Ok(request) => {
                let messages_sent_temp = Arc::clone(&messages_sent);
                std::thread::spawn(move || {
                    handle_request(request, messages_sent_temp);
                })
            }
            Err(error) => {
                println!("error: {}", error);
                break;
            }
        };
    }
}

fn handle_request(mut request: tiny_http::Request, messages_sent: Arc<Mutex<Vec<MessageSent>>>) {
    match request.method() {
        tiny_http::Method::Options => {
            let response = tiny_http::Response::from_string("Ok")
                .with_status_code(200)
                .with_header(create_header("Access-Control-Allow-Origin", "*"))
                .with_header(create_header(
                    "Access-Control-Allow-Methods",
                    "POST, GET, OPTIONS",
                ))
                .with_header(create_header(
                    "Access-Control-Allow-Headers",
                    "Content-Type",
                ));

            let _ = request.respond(response);
        },
        tiny_http::Method::Post => {
            if request.url().contains("/messages") {
                let mut output = String::new();
                request.as_reader().read_to_string(&mut output).unwrap();

                match serde_json::from_str::<MessageSent>(&output) {
                    Ok(message_sent) => {
                        println!("Mensagem recebida via WebSocket: {:?}", message_sent);

                        if let Ok(mut vec) = messages_sent.lock() {
                            vec.push(message_sent);
                        }
                    },
                    Err(e) => {
                        println!("Erro ao desserializar JSON: {}", e);
                    }
                };

                let response = tiny_http::Response::from_string(output)
                    .with_status_code(tiny_http::StatusCode(201))
                    .with_header(create_header("Content-Type", "application/json"));
                let _ = request.respond(response);
            }
        },
        tiny_http::Method::Get => {
            if request.url().contains("/messages") {
                match verify_websocket_connection(&request) {
                    None => {
                        let response = tiny_http::Response::new_empty(tiny_http::StatusCode(405));
                        request.respond(response).unwrap_or_default();
                    }
                    _ => {
                        start_websocket_connection(request, messages_sent);
                    }
                }
                return;
            }
            
            let response = tiny_http::Response::new_empty(tiny_http::StatusCode(405));
            request.respond(response).unwrap_or_default();
        },
        _ => {
            let response = tiny_http::Response::new_empty(tiny_http::StatusCode(405));

            request.respond(response).unwrap_or_default();
        }
    }
}

fn verify_websocket_connection(request: &tiny_http::Request) -> Option<tiny_http::Header> {
    request
        .headers()
        .iter()
        .find(|h| h.field.equiv(&"Upgrade"))
        .and_then(|hdr| {
            if hdr.value == "websocket" {
                Some(hdr.clone())
            } else {
                None
            }
        })
}

fn start_websocket_connection(request: tiny_http::Request, messages_sent: Arc<Mutex<Vec<MessageSent>>>) {
    let key = match request
        .headers()
        .iter()
        .find(|h| h.field.equiv(&"Sec-WebSocket-Key"))
        .map(|h| h.value.clone())
    {
        None => {
            let response = tiny_http::Response::new_empty(tiny_http::StatusCode(400));
            request.respond(response).expect("Responded");
            return;
        },
        Some(k) => k,
    };

    let accept_key = convert_key(key.as_str());
    let response = tiny_http::Response::new_empty(tiny_http::StatusCode(101))
        .with_header(create_header("Update", "websocket"))
        .with_header(create_header("Connection", "Upgrade"))
        .with_header(create_header("Sec-WebSocket-Accept", &accept_key));

    let mut stream = request.upgrade("websocket", response);

    let mut last_sent: usize = 0;
    loop { // Infinite loop, even after client connection closed :sad:
        println!("Running thread");
        // Envia apenas as mensagens novas
        if let Ok(vec) = messages_sent.lock() {
            while last_sent < vec.len() {
                if let Ok(json) = serde_json::to_string(&vec[last_sent]) {
                    let _ = send_websocket_text(&mut stream, &json);
                }
                last_sent += 1;
            }
        }

        // Pequeno delay para evitar busy loop
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    todo!("Infinite loop, even after client connection closed :sad:");
}

fn send_websocket_text<W: Write>(stream: &mut W, msg: &str) -> std::io::Result<()> {
    let payload = msg.as_bytes();
    let payload_len = payload.len();

    let mut frame = Vec::with_capacity(2 + payload_len);
    frame.push(0x81); // FIN + opcode texto

    if payload_len <= 125 {
        frame.push(payload_len as u8);
    } else if payload_len <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(payload_len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(payload_len as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);

    stream.write_all(&frame)?;
    stream.flush()?;
    Ok(())
}