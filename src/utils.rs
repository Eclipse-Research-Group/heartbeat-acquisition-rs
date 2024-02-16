pub trait SingletonService<T, E> {
    fn shutdown() -> Result<(), E>;
    fn start() -> Result<(), E>;
    fn get_service() -> Option<&'static T>;
}