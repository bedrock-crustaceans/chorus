#[macro_export]
macro_rules! modules {
    (
        $(
            $vis:vis $name:ident
        ),* $(,)?
    ) => {
        $(
            $vis mod $name;
        )*
    };
}