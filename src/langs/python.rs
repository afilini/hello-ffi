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

// #[pyo3::prelude::pyproto]
// impl<T> pyo3::class::PyObjectProtocol for T
// where
//     T: ToString
// {
//     fn __str__(&self) -> String {
//         self.to_string()
//     }
// }

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
