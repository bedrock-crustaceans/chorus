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

#[macro_export]
macro_rules! singleton {
    ($name:ident : $ty:ty) => {
        static mut $name: Option<$ty> = None;

        impl $ty {
            pub fn instance() -> &'static mut $ty {
                unsafe { $name.as_mut().expect("Singleton not initialized") }
            }

            pub fn init(value: $ty) {
                unsafe {
                    $name = Some(value);
                }
            }
        }
    };
}

#[macro_export]
macro_rules! reexports {
    ($($path:path),* $(,)?) => {
        $(
            pub use $path;
        )*
    };
}

#[macro_export]
macro_rules! boxed {
    ($value:expr) => {
        Box::new($value)
    };
}

#[macro_export]
macro_rules! arc {
    ($value:expr) => {
        std::sync::Arc::new($value)
    };
}

#[macro_export]
macro_rules! lock {
    ($value:expr) => {
        $value.lock().unwrap()
    };
}
