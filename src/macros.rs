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

macro_rules! erased_downcast {
    ($inner:ident $(, { $($method:ident($($arg:ident: $ty:ty),*) -> $ret:ty);* $(;)? })?) => {
        erased_downcast!($inner<T> $(, { $($method($($arg: $ty),*) -> $ret);* })?);
    };
    ($inner:ident<$($tp:ident),+> $(, { $($method:ident($($arg:ident: $ty:ty),*) -> $ret:ty);* $(;)? })?) => {
        use paste::paste;

        paste! {
            pub(crate) trait [< Erased $inner >]: std::any::Any {
                fn as_any(&self) -> &dyn std::any::Any;
                fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
                $($(fn $method(&mut self $(, $arg: $ty)*) -> $ret;)*)?
            }

            impl<$($tp: 'static),+> [< Erased $inner >] for $inner<$($tp),+> {
                fn as_any(&self) -> &dyn std::any::Any {
                    self
                }

                fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
                    self
                }

                $($(fn $method(&mut self $(, $arg: $ty)*) -> $ret {
                    $inner::$method(self $(, $arg)*)
                })*)?
            }
        }
    };
}

pub(crate) use erased_downcast;

macro_rules! erased_get {
    ($erased:expr, $outer:ident, $key:ty $(, $extra:ty)*) => {
        $erased
            .get(&TypeId::of::<$key>())
            .and_then(|o| o.as_any().downcast_ref::<$outer<$key $(, $extra)*>>())
    };
}

pub(crate) use erased_get;

macro_rules! erased_entry {
    ($erased:expr, $outer:ident, $key:ty $(, $extra:ty)*) => {
        $erased
            .entry(TypeId::of::<$key>())
            .or_insert_with(|| Box::new($outer::<$key $(, $extra)*>::new()))
            .as_any_mut()
            .downcast_mut::<$outer<$key $(, $extra)*>>()
            .expect("TypeId keyed the wrong registry")
    };
}

pub(crate) use erased_entry;
