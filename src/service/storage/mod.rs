use std::{collections::VecDeque, mem::MaybeUninit, sync::{Arc, Mutex, Once, RwLock}};
use anyhow::Result;
use minio::s3::{creds::StaticProvider};
use url::Url;
use minio::s3::args::{BucketExistsArgs, MakeBucketArgs, UploadObjectArgs};
use minio::s3::client::Client;
use minio::s3::http::BaseUrl;
use std::thread;
use crate::utils::SingletonService;

#[derive(Debug, Clone)]
pub struct StorageServiceSettings {
    endpoint: String,
    key: String,
    secret: String
}

impl StorageServiceSettings {
    pub fn new(endpoint: String, key: String, secret: String) -> StorageServiceSettings {
        StorageServiceSettings {
            endpoint,
            key,
            secret
        }
    }
}

pub struct StorageService {
    inner: Arc<Mutex<StorageServiceInner>>
}

static mut SINGLETON: MaybeUninit<StorageService> = MaybeUninit::uninit();

impl StorageService {

    pub fn new(credentials: StorageServiceSettings) -> Result<&'static StorageService> {
        unsafe {
            SINGLETON = MaybeUninit::new(StorageService {
            inner: Arc::new(Mutex::new(StorageServiceInner::new(credentials)?))
            });
        }

        Ok(StorageService::get_service()?)
    }

    pub fn connect(&self) -> Result<()> {
        self.inner.lock().unwrap().connect()?;
        Ok(())
    }
    
    pub fn shutdown_and_wait(&self) -> Result<()> {
        log::info!("Shutting down storage service");
        let mut inner_lock = self.inner.lock().unwrap();
        inner_lock.cancellationToken.cancel();
        let thread = &mut inner_lock.thread;
        let thread = thread.take().unwrap();
        thread.join().unwrap();
        Ok(())
    }

    pub fn queue_upload(&self, args: UploadArgs) -> Result<()> {
        self.inner.lock().unwrap().queue_upload(args)?;
        Ok(())
    }

    pub fn get_service() -> Result<&'static StorageService> {
        static ONCE: Once = Once::new();

        unsafe {
            Ok(SINGLETON.assume_init_ref())
        }
    }
}

#[derive(Debug, Clone)]
pub struct UploadArgs {
    pub file_path: String,
    pub object_name: String
}

struct StorageServiceInner {
    settings: StorageServiceSettings,
    thread: Option<thread::JoinHandle<()>>,
    cancellationToken: tokio_util::sync::CancellationToken,
    upload_queue: Arc<Mutex<VecDeque<UploadArgs>>>
}

impl StorageServiceInner {
    fn new(settings: StorageServiceSettings) -> Result<StorageServiceInner> {
        log::info!("Initializing storage service");

        Ok(StorageServiceInner {
            settings,
            thread: None,
            cancellationToken: tokio_util::sync::CancellationToken::new(),
            upload_queue: Arc::new(Mutex::new(VecDeque::new()))
        })
    }   

    fn connect(&mut self) -> Result<()> {
        let static_provider = StaticProvider::new(
            self.settings.key.as_str(),
            self.settings.secret.as_str(),
            None
        );

        let client = Client::new(
            self.settings.endpoint.parse::<BaseUrl>()?,
            Some(Box::new(static_provider)), 
            None,
            None
        )?;
    
        let token_clone = self.cancellationToken.clone();
        let queue_clone = self.upload_queue.clone();
        let thread = thread::spawn(move || {
            log::debug!("Storage thread spawned.");
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("storage")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_all()
                .build().unwrap();

            runtime.block_on(async {
                let queue = queue_clone;
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                            log::info!("Storage thread is alive.");
                        },
                        _ = token_clone.cancelled() => {
                            log::info!("Safely exiting storage thread.");
                            break;
                        }
                    }             

                    while let Some(upload) = {
                        let mut queue = queue.lock().unwrap();
                        queue.pop_front()
                    } {

                        log::info!("Uploading file: {}", upload.file_path);

                        let args = match UploadObjectArgs::new(
                            "test",
                            upload.object_name.as_str(),
                            upload.file_path.as_str()
                        ) {
                            Ok(args) => args,
                            Err(e) => {
                                log::error!("Error creating upload args: {:?}", e);
                                continue;
                            }
                        };

                        match client.upload_object(
                            &args
                        ).await {
                            Ok(_) => {
                                log::info!("Uploaded file: {}", upload.file_path);
                                log::warn!("NEED TO IMPLEMENT RE_UPLOAD IF FAIL");
                            },
                            Err(e) => {
                                log::error!("Error uploading file: {:?}", e);
                            }
                        }

                    }
                    
                }
            });
 
        });

        self.thread = Some(thread);

        Ok(())
    }

    fn queue_upload(&mut self, args: UploadArgs) -> Result<()> {
        self.upload_queue.lock().unwrap().push_back(args);
        Ok(())
    }
}