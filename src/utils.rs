pub trait SingletonService<T, E> {
    fn shutdown(&self) -> impl std::future::Future<Output = Result<(), E>> + Send;
    fn run(&self) -> impl std::future::Future<Output = Result<(), E>> + Send;
    fn is_alive(&self) -> impl std::future::Future<Output = Result<bool, E>> + Send {
        async { Ok(true) }
    }
    fn get_service() -> Option<&'static T>;
}

pub fn map_lock_error<T>(_e: T) -> anyhow::Error {
    anyhow::anyhow!("Error locking storage service")
}
