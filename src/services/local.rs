use std::{path::PathBuf, sync::{Arc, Mutex}};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::get, Json, Router};
use futures::TryFutureExt;

use crate::serial::Frame;

use super::ServiceMessage;

#[derive(Debug, Clone)]
pub struct LocalServiceConfig {
    pub port: u16,
    pub node_id: String,
}

pub struct LocalService {
    config: LocalServiceConfig,
    last_frame: std::sync::Arc<std::sync::Mutex<Option<crate::serial::Frame>>>,
    tx: tokio::sync::broadcast::Sender<ServiceMessage>,
    watch_tx: tokio::sync::watch::Sender<Option<()>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FrameResponse {
    frame: Option<Frame>,
    node_id: String,
}

impl LocalService {
    pub fn new(config: LocalServiceConfig,
        tx: tokio::sync::broadcast::Sender<ServiceMessage>) -> LocalService {

        let last_frame = std::sync::Arc::new(std::sync::Mutex::new(None));

        let (w_tx, _) = tokio::sync::watch::channel(Option::<()>::None);

        LocalService {
            config, 
            last_frame: last_frame,
            tx: tx,
            watch_tx: w_tx,
        }
    }


    pub async fn start(&mut self) -> anyhow::Result<()> {

        let last_frame_inner = self.last_frame.clone();
        let tx = self.tx.clone();
        tokio::spawn(async move {
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

        let last_frame_inner = self.last_frame.clone();
        let config = self.config.clone();
        let watch_rx = self.watch_tx.subscribe();
        tokio::spawn(async move {
            let router = Router::new()
                .route("/frame", get(Self::get_frame))
                .with_state(last_frame_inner);
            let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", config.port)).await.unwrap();

            axum::serve(listener, router)
                .with_graceful_shutdown(Self::graceful_shutdown_signal(watch_rx))
                .await.unwrap();

            log::info!("Server shutdown");
        });

        Ok(())
    }

    pub async fn graceful_shutdown_signal(mut watch_rx: tokio::sync::watch::Receiver<Option<()>>) {
        watch_rx.changed().await.unwrap();
    }

    pub fn stop(&mut self) {
        self.watch_tx.send(Some(())).unwrap();
    }

    pub async fn get_frame(State(last_frame): State<Arc<Mutex<Option<Frame>>>>) -> impl IntoResponse {
        let last_frame = last_frame.lock().unwrap();
        match last_frame.as_ref() {
            Some(frame) => {
                (StatusCode::OK, Json(FrameResponse {
                    frame: Some(frame.clone()),
                    node_id: "local".to_string(),
                }))
            }
            None => {
                (StatusCode::NOT_FOUND, Json({
                    FrameResponse {
                        frame: None,
                        node_id: "local".to_string(),
                    }
                }))
            }
        }
    }
}