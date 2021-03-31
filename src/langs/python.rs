use std::ops::{Deref, DerefMut};

use pyo3::prelude::*;
use pyo3::pyclass::PyClass;
use pyo3::pyclass_init::PyClassInitializer;
use pyo3::type_object::{PyBorrowFlagLayout, PyTypeInfo};

use crate::common::*;
use crate::mapping::*;

pub struct PyCb<'source>(&'source pyo3::PyAny);

impl<'source> std::ops::Deref for PyCb<'source> {
    type Target = &'source pyo3::PyAny;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'source> pyo3::conversion::FromPyObject<'source> for PyCb<'source> {
    fn extract(ob: &'source pyo3::PyAny) -> pyo3::PyResult<Self> {
        if !ob.is_callable() {
            Err(pyo3::exceptions::PyTypeError::new_err(
                "Argument is not callable",
            ))
        } else {
            Ok(PyCb(ob))
        }
    }
}

pub struct MyVec<T>(pub Vec<T>);

impl<T> std::ops::Deref for MyVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl<T> std::ops::DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
// TODO: impl IntoIterator, FromIterator

impl<T: PyClass> AccessContainer for pyo3::Py<T> {
    type Content = T;

    fn access_container<R, F: Fn(&Self::Content) -> R>(&self, f: F) -> R {
        Python::with_gil(|py| { f(self.as_ref(py).borrow().deref()) })
    }

    fn access_container_mut<R, F: Fn(&mut Self::Content) -> R>(&mut self, f: F) -> R {
        Python::with_gil(|py| { f(self.as_ref(py).borrow_mut().deref_mut()) })
    }
}

#[macro_export]
macro_rules! impl_py_error {
    ($type:ident) => {
        impl Into<pyo3::PyErr> for $type {
            fn into(self) -> pyo3::PyErr {
                // TODO: proper errors
                pyo3::exceptions::PyTypeError::new_err(format!("{:?}", self))
            }
        }
    };
}

pub trait IntoTraitStruct: Sized {
    type Target;

    fn into_trait_struct(self) -> Self::Target;
}

impl<T> WrappedStructField for T
where
    T: ExposedStruct + pyo3::PyTypeInfo + Into<PyClassInitializer<T>> + PyClass,
    <T as PyTypeInfo>::BaseLayout: PyBorrowFlagLayout<<T as PyTypeInfo>::BaseType>,
{
    type Store = Py<T>;

    type Getter = Py<T>;
    type Setter = T;

    fn wrap_get(s: &mut Self::Store) -> Self::Getter {
        Python::with_gil(|py| -> Py<T> { s.clone_ref(py) })
    }

    fn wrap_set(s: Self::Setter) -> Self::Store {
        MapFrom::map_from(s)
    }
}
