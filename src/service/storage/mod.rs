use anyhow::Result;
use futures::lock::Mutex;
use minio::s3::args::UploadObjectArgs;
use minio::s3::client::Client;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use std::process::Command;
use std::thread;
use std::{collections::VecDeque, mem::MaybeUninit, path::PathBuf, sync::Arc};

use crate::utils::{map_lock_error, SingletonService};

pub enum StorageServiceError {}

#[derive(Debug, Clone)]
pub struct StorageServiceSettings {
    endpoint: String,
    key: String,
    secret: String,
    bucket: String,
}

impl StorageServiceSettings {
    pub fn new(
        endpoint: String,
        key: String,
        secret: String,
        bucket: String,
    ) -> StorageServiceSettings {
        StorageServiceSettings {
            endpoint,
            key,
            secret,
            bucket,
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
    inner: Arc<futures::lock::Mutex<StorageServiceInner>>,
}

static mut SINGLETON: MaybeUninit<StorageService> = MaybeUninit::uninit();

impl SingletonService<StorageService, anyhow::Error> for StorageService {
    fn get_service() -> Option<&'static StorageService> {
        if unsafe { SINGLETON.as_ptr().is_null() } {
            None
        } else {
            unsafe { Some(SINGLETON.assume_init_ref()) }
        }
    }

    async fn shutdown(&self) -> Result<()> {
        self.inner.lock().await.shutdown_and_wait().await
    }

    async fn run(&self) -> Result<()> {
        self.inner.lock().await.connect()
    }

    async fn is_alive(&self) -> Result<bool> {
        Ok(self.inner.lock().await.is_alive())
    }
}

impl StorageService {
    pub fn new(credentials: StorageServiceSettings) -> Result<&'static StorageService> {
        unsafe {
            SINGLETON = MaybeUninit::new(StorageService {
                inner: Arc::new(futures::lock::Mutex::new(StorageServiceInner::new(
                    credentials,
                )?)),
            });
        }

        Ok(StorageService::get_service().ok_or(anyhow::anyhow!("Service not initialized"))?)
    }

    pub async fn settings(&self) -> Result<StorageServiceSettings> {
        Ok(self.inner.lock().await.settings.clone())
    }

    pub async fn queue_upload(&self, args: UploadArgs) -> Result<()> {
        self.inner.lock().await.queue_upload(args).await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct UploadArgs {
    pub bucket_name: String,
    pub file_path: String,
    pub object_path: String,
}

impl UploadArgs {
    pub fn new(bucket_name: String, file_path: String, object_path: String) -> Result<UploadArgs> {
        Ok(UploadArgs {
            bucket_name: bucket_name.to_string(),
            file_path: file_path.to_string(),
            object_path: object_path.to_string(),
        })
    }
}

struct StorageServiceInner {
    settings: StorageServiceSettings,
    handle: Option<thread::JoinHandle<()>>,
    cancellation_token: tokio_util::sync::CancellationToken,
    upload_queue: Arc<Mutex<VecDeque<UploadArgs>>>,
}

impl StorageServiceInner {
    fn new(settings: StorageServiceSettings) -> Result<StorageServiceInner> {
        Ok(StorageServiceInner {
            settings,
            handle: None,
            cancellation_token: tokio_util::sync::CancellationToken::new(),
            upload_queue: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    fn gzip_file(file: PathBuf) -> Result<()> {
        let output = Command::new("gzip").arg(file.as_os_str()).output()?;

        if output.status.success() {
            log::debug!("File successfully compressed: {}", file.display());
        } else {
            // If gzip encountered an error, stderr will contain the error message
            let error_message = String::from_utf8_lossy(&output.stderr);
            log::error!("gzip error: {}", error_message);
        }

        Ok(())
    }

    fn get_client(&self) -> Result<Client> {
        let static_provider = StaticProvider::new(
            self.settings.key.as_str(),
            self.settings.secret.as_str(),
            None,
        );

        let client = Client::new(
            self.settings.endpoint.parse::<BaseUrl>()?,
            Some(Box::new(static_provider)),
            None,
            None,
        )?;

        Ok(client)
    }

    fn connect(&mut self) -> Result<()> {
        let client = self.get_client()?;

        let token_clone = self.cancellation_token.clone();
        let queue_clone = self.upload_queue.clone();

        let thread = thread::spawn(move || {
            log::debug!("Storage thread spawned.");
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(4)
                .thread_name("storage")
                .thread_stack_size(3 * 1024 * 1024)
                .enable_all()
                .build()
                .unwrap();

            runtime.block_on(async {
                let queue = queue_clone;
                let client = client;
                loop {
                    tokio::select! {
                        _ = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                            log::debug!("Storage thread is alive.");
                        },
                        _ = token_clone.cancelled() => {
                            log::debug!("Safely exiting storage thread.");
                            break;
                        }
                    }

                    while let Some(upload) = {
                        let mut queue = queue.lock().await;
                        queue.pop_front()
                    } {
                        let client = client.clone();

                        match StorageServiceInner::do_upload(client, &upload).await {
                            Ok(_) => {
                                log::debug!("Upload successful");
                            }
                            Err(e) => {
                                log::error!("Error uploading file: {:?}", e);
                                let mut queue = queue.lock().await;
                                queue.push_back(upload);
                                break;
                            }
                        }
                    }
                }
            });
        });

        self.handle = Some(thread);

        Ok(())
    }

    async fn do_upload(client: Client, upload: &UploadArgs) -> Result<()> {
        log::info!(
            "Uploading file {} to {}",
            upload.file_path,
            upload.object_path
        );

        let args = match UploadObjectArgs::new(
            upload.bucket_name.as_str(),
            upload.object_path.as_str(),
            upload.file_path.as_str(),
        ) {
            Ok(args) => args,
            Err(e) => {
                return Err(anyhow::anyhow!("Error creating upload args: {:?}", e));
            }
        };

        match client.upload_object(&args).await {
            Ok(_) => {
                log::info!("Upload successful");
                match Self::gzip_file(PathBuf::from(upload.file_path.clone())) {
                    Ok(_) => {
                        log::debug!("File compressed successfully");
                        return Ok(());
                    }
                    Err(e) => {
                        log::error!("Error compressing file: {:?}", e);
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Error uploading file: {:?}", e));
            }
        };
    }

    async fn queue_upload(&mut self, args: UploadArgs) -> Result<()> {
        self.upload_queue.lock().await.push_back(args);
        Ok(())
    }

    async fn shutdown_and_wait(&mut self) -> Result<()> {
        let client = self.get_client()?;
        let queue = self.upload_queue.clone();

        self.cancellation_token.cancel();
        let thread = self.handle.take();
        if let Some(thread) = thread {
            thread
                .join()
                .map_err(|e| anyhow::anyhow!("Error joining thread: {:?}", e))?;
        } else {
            log::warn!("No active thread found!");
        }

        // Try to upload remaining files
        log::warn!("MAY NOT NEED TO UPLOAD HERE");
        // thread::spawn(move || {
        //     log::debug!("Storage thread spawned.");
        //     let runtime = tokio::runtime::Builder::new_multi_thread()
        //         .worker_threads(4)
        //         .thread_name("storage")
        //         .thread_stack_size(3 * 1024 * 1024)
        //         .enable_all()
        //         .build()
        //         .unwrap();

        //     runtime.block_on(async {
        //         while let Some(upload) = {
        //             let mut queue = queue.lock().await;
        //             queue.pop_front()
        //         } {
        //             log::info!(
        //                 "Uploading file {} to {}",
        //                 upload.file_path,
        //                 upload.object_path
        //             );

        //             let args = match UploadObjectArgs::new(
        //                 upload.bucket_name.as_str(),
        //                 upload.object_path.as_str(),
        //                 upload.file_path.as_str(),
        //             ) {
        //                 Ok(args) => args,
        //                 Err(e) => {
        //                     // return Err(anyhow::anyhow!("Error creating upload args: {:?}", e));
        //                     log::error!("Error creating upload args: {:?}", e);
        //                     continue;
        //                 }
        //             };

        //             let client = client.clone();

        //             match StorageServiceInner::do_upload(client, &upload).await {
        //                 Ok(_) => {
        //                     log::debug!("Upload successful");
        //                 }
        //                 Err(e) => {
        //                     log::error!("Error uploading file: {:?}", e);
        //                     break;
        //                 }
        //             }
        //         }
        //     });
        // })
        // .join()
        // .unwrap();

        Ok(())
    }

    fn is_alive(&self) -> bool {
        if self.handle.is_some() {
            self.handle.as_ref().unwrap().is_finished()
        } else {
            false
        }
    }
}
