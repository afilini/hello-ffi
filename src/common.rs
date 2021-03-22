pub(crate) trait Sealed {}

#[macro_export]
macro_rules! wrap_struct {
    ($ident:ident, $ty:ty) => {
        impl From<$ty> for $ident {
            fn from(inner: $ty) -> Self {
                $ident { inner }
            }
        }
        impl crate::common::Sealed for $ident {}
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
    };
}
