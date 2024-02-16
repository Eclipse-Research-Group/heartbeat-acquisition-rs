use std::{collections::VecDeque, mem::MaybeUninit, sync::{Arc, Mutex}};
use anyhow::Result;
use minio::s3::creds::StaticProvider;
use minio::s3::args::UploadObjectArgs;
use minio::s3::client::Client;
use minio::s3::http::BaseUrl;
use std::thread;

#[derive(Debug, Clone)]
pub struct StorageServiceSettings {
    endpoint: String,
    key: String,
    secret: String,
    bucket: String
}

impl StorageServiceSettings {
    pub fn new(endpoint: String, key: String, secret: String, bucket: String) -> StorageServiceSettings {
        StorageServiceSettings {
            endpoint,
            key,
            secret,
            bucket
        }
    }

    pub fn bucket(&self) -> String {
        self.bucket.clone()
    }

    pub fn endpoint(&self) -> String {
        self.endpoint.clone()
    }

    pub fn key(&self) -> String {
        self.key.clone()
    }

    pub fn secret(&self) -> String {
        self.secret.clone()
    }
}

pub struct StorageService {
    inner: Arc<Mutex<StorageServiceInner>>
}

static mut SINGLETON: MaybeUninit<StorageService> = MaybeUninit::uninit();

fn lock_error<T>(_e: T) -> anyhow::Error {
    anyhow::anyhow!("Error locking storage service")
}

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
        self.inner.lock()
            .map_err(lock_error)?
            .connect()?;
        Ok(())
    }

    pub fn settings(&self) -> Result<StorageServiceSettings> {
        Ok(self.inner.lock()
            .map_err(lock_error)?
            .settings.clone())
    }
    
    pub fn shutdown_and_wait(&self) -> Result<()> {
        log::info!("Shutting down storage service");
        self.inner.lock()
            .map_err(lock_error)?
            .shutdown_and_wait()?;
        Ok(())
    }

    pub fn queue_upload(&self, args: UploadArgs) -> Result<()> {
        self.inner.lock()
            .map_err(lock_error)?
            .queue_upload(args)?;
        Ok(())
    }

    pub fn get_service() -> Result<&'static StorageService> {
        if unsafe { SINGLETON.as_ptr().is_null() } {
            Err(anyhow::anyhow!("Storage service not initialized"))
        } else {
            unsafe {
                Ok(SINGLETON.assume_init_ref())
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct UploadArgs {
    pub bucket_name: String,
    pub file_path: String,
    pub object_path: String
}

impl UploadArgs {
    pub fn new(bucket_name: String, file_path: String, object_path: String) -> Result<UploadArgs> {
        Ok(UploadArgs {
            bucket_name: bucket_name.to_string(),
            file_path: file_path.to_string(),
            object_path: object_path.to_string()
        })
    }
}

struct StorageServiceInner {
    settings: StorageServiceSettings,
    thread: Option<thread::JoinHandle<()>>,
    cancellation_token: tokio_util::sync::CancellationToken,
    upload_queue: Arc<Mutex<VecDeque<UploadArgs>>>
}

impl StorageServiceInner {
    fn new(settings: StorageServiceSettings) -> Result<StorageServiceInner> {
        log::info!("Initializing storage service");

        Ok(StorageServiceInner {
            settings,
            thread: None,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
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
    
        let token_clone = self.cancellation_token.clone();
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

                        log::info!("Uploading file {} to {}", upload.file_path, upload.object_path);

                        let args = match UploadObjectArgs::new(
                            upload.bucket_name.as_str(),
                            upload.object_path.as_str(),
                            upload.file_path.as_str()
                        ) {
                            Ok(args) => args,
                            Err(e) => {
                                log::error!("Error creating upload args: {:?}", e);
                                let mut queue = queue.lock().unwrap();
                                queue.push_back(upload);
                                break;
                            }
                        };

                        tokio::select! {
                            result = client.upload_object(
                                &args
                            ) => {
                                match result {
                                    Ok(_) => {
                                        log::info!("Upload successful");
                                    },
                                    Err(e) => {
                                        log::error!("Error uploading file: {:?}", e);
                                        let mut queue = queue.lock().unwrap();
                                        queue.push_back(upload);
                                        break;
                                    }
                                }
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
        self.upload_queue.lock()
            .map_err(lock_error)?
            .push_back(args);
        Ok(())
    }

    fn shutdown_and_wait(&mut self) -> Result<()> {
        self.cancellation_token.cancel();
        let thread = self.thread.take();
        if let Some(thread) = thread {
            thread.join().map_err(|e| anyhow::anyhow!("Error joining thread: {:?}", e))?;
        } else {
            log::warn!("No active thread found!");
        }

        Ok(())
    }
}