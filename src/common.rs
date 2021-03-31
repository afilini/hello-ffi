pub trait ExposedStruct {}

pub trait IntoWrapped {
    type Target;
    fn into_wrapped(self) -> Self::Target;
}

pub trait WrappedStructField {
    type Store;

    type Getter;
    type Setter;

    fn wrap_get(s: &mut Self::Store) -> Self::Getter;

    fn wrap_set(s: Self::Setter) -> Self::Store;
}

pub trait AccessContainer {
    type Content;

    /// Access a container and pass a reference to its content to the closure
    fn access_container<R, F: Fn(&Self::Content) -> R>(&self, f: F) -> R;

    /// Same as `access_container()` but mutably
    fn access_container_mut<R, F: Fn(&mut Self::Content) -> R>(&mut self, f: F) -> R;
}

macro_rules! impl_native_wrapper_struct_field {
    ($ty:ty) => {
        impl WrappedStructField for $ty {
            type Store = $ty;

            type Getter = $ty;
            type Setter = $ty;

            #[inline]
            fn wrap_get(s: &mut Self::Store) -> Self::Getter {
                *s
            }

            #[inline]
            fn wrap_set(s: Self::Setter) -> Self::Store {
                s
            }
        }
    }
}
impl_native_wrapper_struct_field!(i8);
impl_native_wrapper_struct_field!(u8);
impl_native_wrapper_struct_field!(i16);
impl_native_wrapper_struct_field!(u16);
impl_native_wrapper_struct_field!(i32);
impl_native_wrapper_struct_field!(u32);
impl_native_wrapper_struct_field!(i64);
impl_native_wrapper_struct_field!(u64);

#[macro_export]
macro_rules! wrap_struct {
    ($ident:ident, $ty:ty) => {
        impl From<$ty> for $ident {
            fn from(inner: $ty) -> Self {
                $ident { inner }
            }
        }
        impl crate::common::IntoWrapped for $ty {
            type Target = $ident;

            fn into_wrapped(self) -> Self::Target {
                self.into()
            }
        }
        impl std::ops::Deref for $ident {
            type Target = $ty;

            fn deref(&self) -> &Self::Target {
                &self.inner
            }
        }
        impl std::ops::DerefMut for $ident {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.inner
            }
        }
        impl $ident {
            #[allow(dead_code)]
            pub(crate) fn into_inner(self) -> $ty {
                self.inner
            }
        }
        impl Clone for $ident {
            fn clone(&self) -> Self {
                self.deref().clone().into()
            }
        }
    };
}
