use std::{future::IntoFuture, mem::MaybeUninit, sync::{Arc, Mutex, Once}, thread};
use axum::{
    http::StatusCode, response::IntoResponse, routing::get, Json, Router
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{capture::DataPoint, utils::{map_lock_error, SingletonService}};

use super::status::StatusService;



pub struct WebService {
    inner: Arc<Mutex<WebServiceInner>>
}

impl Clone for WebService {
    fn clone(&self) -> WebService {
        WebService {
            inner: self.inner.clone()
        }
    }
}

impl SingletonService<WebService, anyhow::Error> for WebService {
    fn get_service() -> Option<&'static WebService> {
        static mut SINGLETON: MaybeUninit<WebService> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                // Make it
                let singleton = WebService::new();
                // Store it to the static var, i.e. initialize it
                SINGLETON.write(singleton);
            });

            Some(SINGLETON.assume_init_ref())

        }

        
    }

    fn shutdown(&self) -> Result<(), anyhow::Error> {
        Ok(self.inner.lock().map_err(map_lock_error)?.shutdown())
    }

    fn run(&self) -> Result<(), anyhow::Error> {
        Ok(self.inner.lock().map_err(map_lock_error)?.start())
    }
}

impl WebService {
    pub fn new() -> WebService {
        WebService {
            inner: Arc::new(Mutex::new(WebServiceInner::new()))
        }
    }
}


#[derive(Deserialize, Serialize)]
struct LastDataResponse {
    data: DataPoint
}



struct WebServiceInner {
    cancellation_token: tokio_util::sync::CancellationToken
}

impl WebServiceInner {
    fn new() -> WebServiceInner {

        WebServiceInner {
            cancellation_token: tokio_util::sync::CancellationToken::new()
        }
    }

    async fn get_root() -> (StatusCode, Json<Value>) {
        (StatusCode::OK, Json(json!({"message": "Hello, World!"})))
    }

    async fn get_metrics() -> impl IntoResponse {
        match StatusService::get_service() {
            Some(service) => (StatusCode::OK, service.prometheus_encode()),
            None => return (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error".to_string())
        }
    }

    async fn get_last_data() -> impl IntoResponse {
        match StatusService::get_service() {
            Some(service) => (StatusCode::OK, Json(Some(LastDataResponse { 
                data: service.get_data()
            }))),
            None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(None))
        }
    }

    // async fn get_last_data() -> (StatusCode, Json<LastDataResponse>) {
    //     (StatusCode::OK, Json(LastDataResponse { 
    //         data: StatusService::get_service().unwrap().get_data()
    //     }))
    // }

    pub fn shutdown(&self) {
        self.cancellation_token.cancel();
    }

    pub fn start(&self) -> () {
        log::debug!("Starting web service...");

        let app: Router<()> = Router::new()
            .route("/", get(WebServiceInner::get_root))
            .route("/metrics", get(WebServiceInner::get_metrics))
            .route("/last_data", get(WebServiceInner::get_last_data));

        let cancellation_token = self.cancellation_token.clone();
        thread::spawn(move || {
            log::debug!("Thread spawned.");
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("my-custom-name")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_io()
                .build().unwrap();

            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("0.0.0.0:8003").await.unwrap();
                log::info!("Web service listening on 0.0.0.0:8003");

                tokio::select! {
                    _ = axum::serve(listener, app).into_future() => {
                        log::error!("Web service exited!");
                    },
                    _ = cancellation_token.cancelled() => {
                        log::debug!("Web service cancelled!");
                    }
                };

            });
 
        });
    }
    
}