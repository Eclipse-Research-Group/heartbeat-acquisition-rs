use std::{mem::MaybeUninit, sync::{Arc, Mutex, Once, RwLock}};
use anyhow::Result;
use minio::s3::{creds::StaticProvider};
use url::Url;
use minio::s3::args::{BucketExistsArgs, MakeBucketArgs, UploadObjectArgs};
use minio::s3::client::Client;
use minio::s3::http::BaseUrl;
use std::thread;
use crate::utils::SingletonService;

#[derive(Debug, Clone)]
pub struct StorageServiceCredentials {
    endpoint: String,
    key: String,
    secret: String
}

impl StorageServiceCredentials {
    pub fn new(endpoint: String, key: String, secret: String) -> StorageServiceCredentials {
        StorageServiceCredentials {
            endpoint,
            key,
            secret
        }
    }
}

pub struct StorageService {
    inner: Arc<RwLock<StorageServiceInner>>
}

static mut SINGLETON: MaybeUninit<StorageService> = MaybeUninit::uninit();

impl StorageService {

    pub fn new(credentials: StorageServiceCredentials) -> Result<&'static StorageService> {
        unsafe {
            SINGLETON = MaybeUninit::new(StorageService {
            inner: Arc::new(RwLock::new(StorageServiceInner::new(credentials)?))
            });
        }

        Ok(StorageService::get_service()?)
    }

    pub fn connect(&self) -> Result<()> {
        self.inner.write().unwrap().connect()?;
        Ok(())
    }

    pub fn get_service() -> Result<&'static StorageService> {
        static ONCE: Once = Once::new();

        unsafe {
            Ok(SINGLETON.assume_init_ref())
        }
    }
}


struct StorageServiceInner {
    credentials: StorageServiceCredentials
}

impl StorageServiceInner {
    fn new(credentials: StorageServiceCredentials) -> Result<StorageServiceInner> {
        log::info!("Initializing storage service");

        Ok(StorageServiceInner {
            credentials: credentials
        })
    }   

    fn connect(&mut self) -> Result<()> {
        let static_provider = StaticProvider::new(
            self.credentials.key.as_str(),
            self.credentials.secret.as_str(),
            None
        );


        let client = Client::new(
            self.credentials.endpoint.parse::<BaseUrl>()?,
            Some(Box::new(static_provider)), 
            None,
            None
        )?;

        thread::spawn(move || {
            log::info!("Thread spawned.");
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("my-custom-name")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_all()
                .build().unwrap();
            runtime.block_on(async {
                log::info!("YOY");
                
                client.bucket_exists(&BucketExistsArgs::new("heartbeat-data").unwrap()).await.unwrap();
                // client.make_bucket(&MakeBucketArgs::new("heartbeat-data")).await.unwrap();

            });
 
        });

        Ok(())
    }
}