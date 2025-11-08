#[macro_export(local_inner_macros)]
macro_rules! define_handle {
    ($name:ident) => {
        #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]  
        pub struct $name(u64);

        impl $name {
            pub(crate) fn new() -> Self {
                let id = Self::counter().fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Self(id)
            }

            fn counter() -> &'static std::sync::atomic::AtomicU64 {
                static COUNTER: std::sync::OnceLock<std::sync::atomic::AtomicU64> = std::sync::OnceLock::new();
                COUNTER.get_or_init(|| std::sync::atomic::AtomicU64::new(1))
            }
        }
    };
}
