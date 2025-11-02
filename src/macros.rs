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

#[macro_export(local_inner_macros)]
macro_rules! define_wrapper {
    (
        $name:ident,
        $trait_name:ident,
        {
            $($funcs:tt)*
        }
    ) => {
        // Generate the trait
        pub(crate) trait $trait_name {
            define_wrapper!(@parse_trait $($funcs)*);
        }

        // Wrapper struct
        pub struct $name<'a>{
            backend: &'a dyn $trait_name,
        }

        impl<'a> $name<'a> {
            pub(crate) fn new(backend: Box<dyn $trait_name>) -> Self {
                Self { backend }
            }

            define_wrapper!(@parse_wrapper $($funcs)*);
        }
    };

    (@parse_trait fn $fname:ident(&self $(, $args:ident : $typ:ty)*) -> $ret:ty; $($tail:tt)*) => {
        fn $fname(&self $(, $args: $typ)*) -> $ret;
        define_wrapper!(@parse_trait $($tail)*);
    };
    (@parse_trait fn $fname:ident(&mut self $(, $args:ident : $typ:ty)*) -> $ret:ty; $($tail:tt)*) => {
        fn $fname(&mut self $(, $args: $typ)*) -> $ret;
        define_wrapper!(@parse_trait $($tail)*);
    };
    (@parse_trait) => {};

    (@parse_wrapper fn $fname:ident(&self $(, $args:ident : $typ:ty)*) -> $ret:ty; $($tail:tt)*) => {
        pub fn $fname(&self $(, $args: $typ)*) -> $ret {
            self.backend.$fname($($args),*)
        }
        define_wrapper!(@parse_wrapper $($tail)*);
    };
    (@parse_wrapper fn $fname:ident(&mut self $(, $args:ident : $typ:ty)*) -> $ret:ty; $($tail:tt)*) => {
        pub fn $fname(&mut self $(, $args: $typ)*) -> $ret {
            self.backend.$fname($($args),*)
        }
        define_wrapper!(@parse_wrapper $($tail)*);
    };
    (@parse_wrapper) => {};
}
