#[macro_export(local_inner_macros)]
macro_rules! define_handle {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]  
        pub struct $name(u64);

        impl $name {
            pub(crate) fn new() -> Self { Self(1) }
            pub(crate) fn next(&mut self) -> Self {
                let handle = self.0 + 1;
                self.0 += 1;
                Self(handle)
            }
        }
    };
}
