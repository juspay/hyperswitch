#[macro_export]
macro_rules! newtype_impl {
    ($is_pub:vis, $name:ident, $ty_path:path) => {
        impl std::ops::Deref for $name {
            type Target = $ty_path;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl From<$ty_path> for $name {
            fn from(ty: $ty_path) -> Self {
                Self(ty)
            }
        }

        impl $name {
            pub fn into_inner(self) -> $ty_path {
                self.0
            }
        }
    };
}

#[macro_export]
macro_rules! newtype {
    ($is_pub:vis $name:ident = $ty_path:path) => {
        $is_pub struct $name(pub $ty_path);

        $crate::newtype_impl!($is_pub, $name, $ty_path);
    };

    ($is_pub:vis $name:ident = $ty_path:path, derives = ($($trt:path),*)) => {
        #[derive($($trt),*)]
        $is_pub struct $name(pub $ty_path);

        $crate::newtype_impl!($is_pub, $name, $ty_path);
    };
}

#[macro_export]
macro_rules! async_spawn {
    ($t:block) => {
        tokio::spawn(async move { $t });
    };
}
