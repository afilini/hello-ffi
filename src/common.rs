pub(crate) mod sealed {
    pub trait Sealed {}
}

pub trait WrappedStructField: Sized {
    type Store;

    type Getter;
    type Setter;

    // TODO: add default impl if Self is Clone
    fn wrap_get(s: &mut Self::Store) -> Self::Getter;

    fn wrap_set(s: Self::Setter) -> Self::Store;
}

pub struct GetterSetterWrapper<T: WrappedStructField>(pub <T as WrappedStructField>::Store);

impl<T: WrappedStructField> GetterSetterWrapper<T> {
    pub fn get(&mut self) -> <T as WrappedStructField>::Getter {
        T::wrap_get(&mut self.0)
    }

    pub fn set(&mut self, value: <T as WrappedStructField>::Setter) {
        self.0 = T::wrap_set(value);
    }
}
#[macro_export]
macro_rules! wrap_struct {
    ($ident:ident, $ty:ty) => {
        impl From<$ty> for $ident {
            fn from(inner: $ty) -> Self {
                $ident { inner }
            }
        }
        impl crate::common::sealed::Sealed for $ident {}
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

#[cfg(feature = "c")]
mod c_common {
    use super::*;

    impl<T> WrappedStructField for T {
        type Store = Box<T>;

        type Getter = *mut T;
        type Setter = T;

        fn wrap_get(s: &mut Self::Store) -> Self::Getter {
            &*s as *mut T;
        }

        fn wrap_set(s: Self::Setter) -> Self::Store {
            Box::new(s)
        }
    }
}

#[cfg(feature = "python")]
mod python_common {
    use pyo3::prelude::*;
    use pyo3::pyclass::PyClass;
    use pyo3::pyclass_init::PyClassInitializer;
    use pyo3::type_object::{PyBorrowFlagLayout, PyTypeInfo};

    use super::*;

    impl<T: PyClass> WrappedStructField for T
    where
        T: pyo3::PyTypeInfo + Into<PyClassInitializer<T>> + PyClass,
        <T as PyTypeInfo>::BaseLayout: PyBorrowFlagLayout<<T as PyTypeInfo>::BaseType>,
    {
        type Store = Py<T>;

        type Getter = Py<T>;
        type Setter = T;

        fn wrap_get(s: &mut Self::Store) -> Self::Getter {
            use pyo3::prelude::*;

            Python::with_gil(|py| -> Py<T> { s.clone_ref(py) })
        }

        fn wrap_set(s: Self::Setter) -> Self::Store {
            crate::mapping::MapFrom::map_from(s)
        }
    }
}
