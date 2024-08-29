use std::{path::PathBuf, sync::{Arc, Mutex}};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};

use crate::serial::Frame;

use super::ServiceMessage;

#[derive(Debug, Clone)]
pub struct LocalServiceConfig {
    pub port: u16,
}

pub struct LocalService {
    config: LocalServiceConfig,
    last_frame: std::sync::Arc<std::sync::Mutex<Option<crate::serial::Frame>>>,
    token: tokio_util::sync::CancellationToken,
    msg_handle: Option<tokio::task::JoinHandle<()>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    tx: tokio::sync::broadcast::Sender<ServiceMessage>,
}

impl LocalService {
    pub fn new(config: LocalServiceConfig,
        tx: tokio::sync::broadcast::Sender<ServiceMessage>) -> LocalService {

        let last_frame = std::sync::Arc::new(std::sync::Mutex::new(None));

        LocalService {
            config, 
            last_frame: last_frame,
            token: tokio_util::sync::CancellationToken::new(),
            msg_handle: None,
            server_handle: None,
            tx: tx,
        }
    }


    pub async fn start(&mut self) -> anyhow::Result<()> {

        let last_frame_inner = self.last_frame.clone();
        let tx = self.tx.clone();
        let msg_handle = tokio::spawn(async move {
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
        self.msg_handle.replace(msg_handle);

        let last_frame_inner = self.last_frame.clone();
        let config = self.config.clone();
        let server_handle = tokio::spawn(async move {
            let router = Router::new()
                .route("/frame", get(Self::get_frame))
                .with_state(last_frame_inner);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await.unwrap();

            axum::serve(listener, router).await.unwrap();
        });

        self.server_handle.replace(server_handle);

        Ok(())
    }

    pub fn stop(&mut self) {
        self.token.cancel();
        if let Some(handle) = self.msg_handle.take() {
            tokio::task::block_in_place(|| {
                handle.abort();
            });
        }
        if let Some(handle) = self.server_handle.take() {
            tokio::task::block_in_place(|| {
                handle.abort();
            });
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