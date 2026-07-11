macro_rules! wrap_glam {
    ($name:ident, $inner:ty) => {
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
        pub struct $name(pub(crate) $inner);

        impl From<$inner> for $name {
            fn from(v: $inner) -> Self {
                Self(v)
            }
        }

        impl From<$name> for $inner {
            fn from(v: $name) -> Self {
                v.0
            }
        }

        impl std::ops::Add for $name {
            type Output = $name;
            fn add(self, rhs: $name) -> $name {
                $name(self.0 + rhs.0)
            }
        }

        impl std::ops::Sub for $name {
            type Output = $name;
            fn sub(self, rhs: $name) -> $name {
                $name(self.0 - rhs.0)
            }
        }

        impl std::ops::Mul<f32> for $name {
            type Output = $name;
            fn mul(self, rhs: f32) -> $name {
                $name(self.0 * rhs)
            }
        }
    };
}

macro_rules! forward_self {
    ($name:ident :: $method:ident) => {
        impl $name {
            pub fn $method(self) -> Self {
                Self(self.0.$method())
            }
        }
    };
}

wrap_glam!(Mat3, glam::Mat3);
wrap_glam!(Mat4, glam::Mat4);
wrap_glam!(Quat, glam::Quat);
wrap_glam!(Vec2, glam::Vec2);
wrap_glam!(Vec3, glam::Vec3);

forward_self!(Mat4::inverse);
