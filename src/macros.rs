#[macro_export]
macro_rules! define_handle {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]  
        pub struct $name(u64);

        impl $name {
            pub(crate) fn new() -> Self { Self(1) }
            pub(crate) fn next(self) -> Self { Self(self.0 + 1) }
        }
    };
}
