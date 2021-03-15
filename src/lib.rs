use derive::{expose_fn, expose_mod, expose_struct};

#[cfg(not(any(feature = "c", feature = "python")))]
compile_error!("No language enabled");

#[cfg(all(feature = "c", any(feature = "python")))]
compile_error!("Enable at most one language");
#[cfg(all(feature = "python", any(feature = "c")))]
compile_error!("Enable at most one language");

pub trait MapFrom<Source> {
    fn map_from(s: Source) -> Self;
}

impl<T> MapFrom<T> for T {
    #[inline]
    fn map_from(s: T) -> Self {
        s
    }
}

pub trait MapTo<Target> {
    fn map_to(self) -> Target;
}

impl<T> MapTo<T> for T {
    #[inline]
    fn map_to(self) -> T {
        self
    }
}

pub trait IntoPlatformError {
    type TargetType: std::fmt::Debug;

    fn into_platform_error(self) -> Self::TargetType;

    fn ok() -> Self::TargetType;
}

#[cfg(feature = "c")]
mod c_mapping {
    use super::{MapFrom, MapTo};

    impl MapFrom<*const libc::c_char> for String {
        fn map_from(s: *const libc::c_char) -> Self {
            unsafe {
                std::ffi::CStr::from_ptr(s)
                    .to_str()
                    .expect("Invalid incoming string")
                    .to_string()
            }
        }
    }

    impl<F: Clone, T: MapFrom<F>> MapFrom<(*const F, usize)> for Vec<T> {
        fn map_from((ptr, len): (*const F, usize)) -> Self {
            let slice = unsafe { std::slice::from_raw_parts(ptr, len) };
            slice.to_vec().into_iter().map(T::map_from).collect()
        }
    }

    impl MapTo<*mut libc::c_char> for String {
        fn map_to(self) -> *mut libc::c_char {
            let cstring = std::ffi::CString::new(self).expect("Invalid outgoing string");
            let ptr = cstring.as_ptr();
            std::mem::forget(cstring);

            ptr as *mut libc::c_char
        }
    }

    impl<F: Clone, T: MapTo<F>> MapTo<(*mut F, usize)> for Vec<T> {
        fn map_to(self) -> (*mut F, usize) {
            let mut mapped: Vec<F> = self.into_iter().map(T::map_to).collect();
            mapped.shrink_to_fit();

            let result = (mapped.as_mut_ptr(), mapped.len());
            std::mem::forget(mapped);

            result
        }
    }
}

#[cfg(feature = "c")]
mod c_destroy {
    pub struct Destroy<T>(*mut T);

    impl<T> std::ops::Drop for Destroy<T> {
        fn drop(&mut self) {
            let _inner = unsafe { Box::from_raw(self.0) };
        }
    }

    impl<T> super::MapFrom<*mut T> for Destroy<T> {
        #[inline]
        fn map_from(ptr: *mut T) -> Self {
            Destroy(ptr)
        }
    }
}

#[cfg(feature = "c")]
mod c_trait_utils {
    #[inline]
    pub fn take_ptr<I>(this: *mut libc::c_void) -> Box<I> {
        unsafe { Box::from_raw(this as *mut I) }
    }
    pub trait IntoTraitStruct<T> {
        fn into_trait_struct(self) -> T;
    }
}

#[cfg(feature = "python")]
mod python_callback {
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
}

#[expose_mod]
mod hello {
    // #[expose_mod]
    // mod inner {
    // }

    // #[expose_struct("opaque")]
    // pub struct HelloStruct {
    //     inner: hello::HelloStruct
    // }

    // #[expose_impl]
    // impl HelloStruct {
    //     #[constructor]
    //     fn hello_struct_new(init: String) -> Self {
    //         HelloStruct {
    //             inner: hello::HelloStruct {
    //                 init
    //             }
    //         }
    //     }

    //     fn hello_static(a: String) -> String {
    //         hello::HelloStruct::hello_static(a.as_str()).to_string()
    //     }

    //     #[destructor]
    //     fn hello_struct_destroy(_s: Self) {
    //     }

    //     fn hello_method(&self, a: String) -> String {
    //         self.inner.hello_method(a.as_str())
    //     }

    //     fn get_init(&self) -> String {
    //         self.inner.init.clone()
    //     }
    // }

    // #[expose_fn]
    // fn test_callback(f: fn(s: String, v: Vec<String>, u: u32) -> String) -> String {
    //     let result = f(
    //         "teststring".to_string(),
    //         vec![String::from("test1"), String::from("test2")],
    //         42,
    //     );
    //     println!("Printing from Rust: {}", result);

    //     result
    // }

    #[derive(Debug)]
    pub enum MyError {
        StringTooLong,
    }

    #[cfg(feature = "python")]
    impl Into<pyo3::PyErr> for MyError {
        fn into(self) -> pyo3::PyErr {
            pyo3::exceptions::PyTypeError::new_err("Error message")
        }
    }

    impl crate::IntoPlatformError for MyError {
        type TargetType = i32;

        fn into_platform_error(self) -> Self::TargetType {
            -1
        }

        fn ok() -> Self::TargetType {
            0
        }
    }

    #[expose_fn]
    fn test_pure_fn(f: Vec<String>) -> Result<Vec<u32>, MyError> {
        println!("Printing from Rust: {}", f[0]);

        if f[0].len() > 5 {
            Err(MyError::StringTooLong)
        } else {
            Ok(vec![0, 1, 2])
        }
    }

    // External crate
    // pub trait MyTrait {
    //     fn method(&self);
    // }
    // struct ImplFromLib(u64);
    // impl MyTrait for ImplFromLib {
    //     fn method(&self) {
    //         println!("Called `method()` on `ImplFromLib({})`", self.0);
    //     }
    // }

    // // Defined manually
    // // #[expose_trait]
    // pub trait MyTraitWrapper {
    //     fn _wrapper_method(&self);
    // }
    // impl<T: MyTrait> MyTraitWrapper for T {
    //     fn _wrapper_method(&self) {
    //         self.method()
    //     }
    // }
    // #[expose_impl]
    // impl MyTraitStruct {
    //     fn my_trait_impl_from_lib_new(value: u64) -> Self {
    //         use crate::c_trait_utils::IntoTraitStruct;

    //         ImplFromLib(value).into_trait_struct()
    //     }
    // }

    // // Automatically generated
    // #[expose_struct("opaque")]
    // pub struct MyTraitStruct {
    //     _wrapper_method: Box<dyn Fn(*mut libc::c_void)>,

    //     this: *mut libc::c_void,
    //     destroy: Box<dyn Fn(*mut libc::c_void)>,
    // }
    // trait IntoMyTraitStruct {
    //     fn into_my_trait_struct(self) -> MyTraitStruct;
    // }
    // impl MyTraitWrapper for MyTraitStruct {
    //     fn _wrapper_method(&self) {
    //         // arg conv
    //         let __output = (self._wrapper_method)(self.this);
    //         // output conv
    //     }
    // }
    // #[expose_impl]
    // impl MyTraitStruct {
    //     #[constructor]
    //     fn my_trait_new(method: fn(s: *mut libc::c_void), this: *mut libc::c_void, destroy: fn(s: *mut libc::c_void)) -> Self {
    //         MyTraitStruct {
    //             _wrapper_method: Box::new(method),

    //             this,
    //             destroy: Box::new(destroy),
    //         }
    //     }
    //     #[destructor]
    //     fn my_trait_destroy(_s: Self) {
    //     }
    // }
    // impl<T: MyTraitWrapper + 'static> crate::c_trait_utils::IntoTraitStruct<MyTraitStruct> for T {
    //     fn into_trait_struct(self) -> MyTraitStruct {
    //         use crate::c_trait_utils::*;

    //         fn _wrapper_method<I: MyTraitWrapper>(this: *mut libc::c_void) {
    //             let this = take_ptr::<I>(this);
    //             let result = this._wrapper_method();

    //             std::mem::forget(this);
    //             result
    //         }
    //         fn destroy<I>(this: *mut libc::c_void) {
    //             let _this = take_ptr::<I>(this);
    //         }

    //         MyTraitStruct {
    //             _wrapper_method: Box::new(_wrapper_method::<T>),

    //             this: Box::into_raw(Box::new(self)) as *mut libc::c_void,
    //             destroy: Box::new(destroy::<T>),
    //         }
    //     }
    // }
    // impl std::ops::Drop for MyTraitStruct {
    //     fn drop(&mut self) {
    //         // TODO: destroy can be NULL
    //         (self.destroy)(self.this);
    //     }
    // }
}

/*
#[pyclass]
pub struct HelloStruct {
    inner: hello::HelloStruct
}

#[pymethods]
impl HelloStruct {
    #[staticmethod]
    pub fn py_hello_static() -> String {
        hello::HelloStruct::hello_static().to_string()
    }

    pub fn py_hello_method(&self, string: String) -> String {
        self.inner.hello_method(string.as_str())
    }
}

#[pymodule]
fn hello(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<HelloStruct>()?;
    Ok(())
}

impl HelloStruct {
    #[no_mangle]
    pub extern "C" fn hello_static() -> *mut c_char {
        str_out!(hello::HelloStruct::hello_static())
    }

    #[no_mangle]
    pub extern "C" fn hello_method(&self, string: *const c_char) -> *mut c_char {
        str_out!(self.inner.hello_method(str_in!(string)))
    }
}

pub struct Wallet(hello::Wallet);

#[repr(C)]
#[derive(Debug)]
pub struct CoinSelection {
    this: *const c_void,
    fn_do_something: unsafe extern "C" fn(*const c_void, u32) -> u32,

    destroy: unsafe extern "C" fn(*const c_void),
}

pub trait IntoAnyCs {
    fn into_any_cs(self) -> CoinSelection;
}

impl<C: hello::CoinSelectionAlgorithm> IntoAnyCs for C {
    fn into_any_cs(self) -> CoinSelection {
        unsafe extern "C" fn fn_do_something<C: hello::CoinSelectionAlgorithm>(this: *const c_void, val: u32) -> u32 {
            assert_ptr!(this);
            let this = Box::from_raw(this as *mut C);
            let result = this.do_something(val);
            std::mem::forget(this);

            result
        }

        unsafe extern "C" fn destroy<C: hello::CoinSelectionAlgorithm>(this: *const c_void) {
            destroy_ptr!(this as *mut C);
        }

        let this = Box::into_raw(Box::new(self)) as *const c_void;

        CoinSelection {
            this,
            fn_do_something: fn_do_something::<C>,
            destroy: destroy::<C>,
        }
    }
}

impl hello::CoinSelectionAlgorithm for CoinSelection {
    fn do_something(&self, val: u32) -> u32 {
        assert_ptr!(self.fn_do_something);

        unsafe {
            (self.fn_do_something)(self.this, val)
        }
    }
}


#[no_mangle]
pub extern "C" fn triple_cs_new(init: u32) -> CoinSelection {
    hello::TripleCS::new(init).into_any_cs()
}

// impl TripleCS {
//     #[no_mangle]
//     pub extern "C" fn triple_cs_new(init: u32, ptr_out: *mut *mut Self) {
//         set_ptr_out!(ptr_out, TripleCS(hello::TripleCS::new(init)));
//     }
// }

// impl AnyCs {
//     #[no_mangle]
//     pub extern "C" fn new(this: *const c_void, fn_do_something: unsafe extern "C" fn(*const c_void, u32) -> u32, destroy: unsafe extern "C" fn(*const c_void)) -> InnerAnyCs {
//         InnerAnyCs { this, fn_do_something, destroy }
//     }
// }

impl Drop for CoinSelection {
    fn drop(&mut self) {
        if (self.destroy as usize) > 0 {
            println!("Dropping {:?}", self.this);

            unsafe { (self.destroy)(self.this); }
        }
    }
}

impl Wallet {
    #[no_mangle]
    pub extern "C" fn wallet_new(wallet_name: *const c_char, ptr_out: *mut *mut Self) {
        set_ptr_out!(ptr_out, Wallet(hello::Wallet::new(str_in!(wallet_name))));
    }

    #[no_mangle]
    pub extern "C" fn create_tx(&'static self, ptr_out: *mut *mut TxBuilder) {
        set_ptr_out!(ptr_out, TxBuilder(self.0.create_tx().convert_internal_cs(|cs| cs.into_any_cs())));
    }

    #[no_mangle]
    pub extern "C" fn wallet_destroy(this: *mut Self) {
        destroy_ptr!(this);
    }
}

pub struct TxBuilder(hello::TxBuilder<'static, CoinSelection>);

impl TxBuilder {
    #[no_mangle]
    pub extern "C" fn enable_flag(&mut self) {
        self.0.enable_flag();
    }

    #[no_mangle]
    pub extern "C" fn disable_flag(&mut self) {
        self.0.disable_flag();
    }

    #[no_mangle]
    pub extern "C" fn coin_selection(&mut self, new_cs: CoinSelection) {
        *self.0.mut_cs() = new_cs;
    }

    #[no_mangle]
    pub extern "C" fn get_wallet_name(&self) -> *mut c_char {
        str_out!(self.0.get_wallet_name())
    }

    #[no_mangle]
    pub unsafe extern "C" fn finish(&mut self) -> u32 {
        let this: TxBuilder = *Box::from_raw(self);
        this.0.finish()
    }
} */
