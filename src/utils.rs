pub trait SingletonService<T> {
    fn get_service() -> &'static T;
}