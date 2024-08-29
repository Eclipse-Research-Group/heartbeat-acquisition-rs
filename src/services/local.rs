use std::{path::PathBuf, sync::{Arc, Mutex}};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};

use crate::serial::Frame;

use super::ServiceMessage;

pub struct LocalServiceConfig {
    pub port: u16,
}

pub struct LocalService {
    config: LocalServiceConfig,
    last_frame: std::sync::Arc<std::sync::Mutex<Option<crate::serial::Frame>>>,
}

impl LocalService {
    pub fn new(config: LocalServiceConfig,
        tx: tokio::sync::broadcast::Sender<ServiceMessage>) -> LocalService {

        let last_frame = std::sync::Arc::new(std::sync::Mutex::new(None));


        let last_frame_inner = last_frame.clone();
        let handle = tokio::spawn(async move {
            let mut rx = tx.subscribe();
            loop {
                match rx.recv().await {
                    Ok(ServiceMessage::NewFrame(frame)) => {
                        log::debug!("Received new frame");
                        match last_frame_inner.lock() {
                            Ok(mut guard) => {
                                guard.replace(frame);
                            }
                            Err(e) => {
                                log::error!("Unable to lock last_frame: {:?}", e);
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        let last_frame_inner = last_frame.clone();
        tokio::spawn(async move {
            let router = Router::new()
                .route("/frame", get(Self::get_frame))
                .with_state(last_frame_inner);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await.unwrap();

            axum::serve(listener, router).await.unwrap();
        });

        LocalService {
            config, 
            last_frame: last_frame
        }
    }


    // State<Arc<Mutex<Option<Frame>>>>
    pub async fn get_frame(State(last_frame): State<Arc<Mutex<Option<Frame>>>>) -> impl IntoResponse {
        let last_frame = last_frame.lock().unwrap();
        match last_frame.as_ref() {
            Some(frame) => {
                (StatusCode::OK, Json(Some(frame.clone())))
            }
            None => {
                (StatusCode::NOT_FOUND, Json(None))
            }
        }
    }
}