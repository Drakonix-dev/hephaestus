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
                static COUNTER: std::sync::OnceLock<std::sync::atomic::AtomicU64> =
                    std::sync::OnceLock::new();
                COUNTER.get_or_init(|| std::sync::atomic::AtomicU64::new(1))
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "{}::{}", stringify!($name), self.0)
            }
        }
    };
}

pub(crate) use define_handle;
