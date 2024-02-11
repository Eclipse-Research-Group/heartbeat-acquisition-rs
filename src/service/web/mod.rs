use std::{mem::MaybeUninit, sync::{Arc, Mutex, Once}, thread};
use axum::{
    routing::{get, post},
    http::StatusCode,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{capture::DataPoint, utils::SingletonService};

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

impl SingletonService<WebService> for WebService {
    fn get_service() -> &'static WebService {
        static mut SINGLETON: MaybeUninit<WebService> = MaybeUninit::uninit();
        static ONCE: Once = Once::new();

        unsafe {
            ONCE.call_once(|| {
                // Make it
                let singleton = WebService::new();
                // Store it to the static var, i.e. initialize it
                SINGLETON.write(singleton);
            });

            SINGLETON.assume_init_ref()

        }


        
    }
}

impl WebService {
    pub fn new() -> WebService {
        WebService {
            inner: Arc::new(Mutex::new(WebServiceInner::new()))
        }
    }

    pub fn start(&self) {
        self.inner.lock().unwrap().start();
    }
}


#[derive(Deserialize, Serialize)]
struct LastDataResponse {
    data: DataPoint
}



struct WebServiceInner {
}

impl WebServiceInner {
    fn new() -> WebServiceInner {

        WebServiceInner {

        }
    }

    async fn get_root() -> (StatusCode, Json<Value>) {
        (StatusCode::OK, Json(json!({"message": "Hello, World!"})))
    }

    async fn get_metrics() -> String {
        StatusService::get_service().prometheus_encode()
    }

    async fn get_last_data() -> (StatusCode, Json<LastDataResponse>) {
        let data = StatusService::get_service().get_data();
        (StatusCode::OK, Json(LastDataResponse { 
            data: StatusService::get_service().get_data()
        }))
    }

    pub fn start(&self) -> () {
        log::info!("Starting web service...");

        let app: Router<()> = Router::new()
            .route("/", get(WebServiceInner::get_root))
            .route("/metrics", get(WebServiceInner::get_metrics))
            .route("/last_data", get(WebServiceInner::get_last_data));

        thread::spawn(move || {
            log::info!("Thread spawned.");
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("my-custom-name")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_io()
                .build().unwrap();

            runtime.block_on(async {
                let listener = tokio::net::TcpListener::bind("0.0.0.0:8003").await.unwrap();
                log::info!("Web service listening on 0.0.0.0:8003");
                axum::serve(listener, app).await.unwrap();
                log::error!("Web service exited!");
            });
 
        });
    }
    
}