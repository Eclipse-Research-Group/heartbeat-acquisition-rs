pub trait SingletonService<T, E> {
    fn shutdown(&self) -> Result<(), E>;
    fn run(&self) -> Result<(), E>;
    fn is_alive(&self) -> Result<bool, E> {
        Ok(true)
    }
    fn get_service() -> Option<&'static T>;
}

pub fn map_lock_error<T>(_e: T) -> anyhow::Error {
    anyhow::anyhow!("Error locking storage service")
}