use derive::expose_mod;

pub mod mapping;
#[macro_use]
pub mod langs;

#[cfg(not(any(feature = "c", feature = "python")))]
compile_error!("No language enabled");

#[cfg(all(feature = "c", any(feature = "python")))]
compile_error!("Enable at most one language");
#[cfg(all(feature = "python", any(feature = "c")))]
compile_error!("Enable at most one language");

#[macro_use]
mod common;

pub mod bdk_mod;
pub mod bitcoin_mod;

// pub trait MyTrait {
//     fn method(&self, s: String) -> String;
// }
// pub struct ImplMyTrait(pub u32);
// impl MyTrait for ImplMyTrait {
//     fn method(&self, s: String) -> String {
//         println!(
//             "Called `method()` on `ImplMyTrait({})` with s = `{}`",
//             self.0, s
//         );
//
//         format!("ModifiedFromRust({})", s)
//     }
// }

#[expose_mod]
mod test_mod {
    #[expose_struct("opaque")]
    struct Inner {
        val: u32
    }
    impl Clone for Inner {
        fn clone(&self) -> Self {
            Inner { val: self.val }
        }
    }

    #[expose_impl]
    impl Inner {
        #[constructor]
        fn new(val: u32) -> Self {
            Inner {
                val,
            }
        }
    }

    #[expose_struct("opaque")]
    struct Outer {
        #[expose_struct(get, set)]
        inner: Inner,
    }

    #[expose_impl]
    impl Outer {
        #[constructor]
        fn new(inner: &Inner) -> Self {
            let inner = inner.clone();

            #[cfg(feature = "python")]
            let inner = pyo3::Py::new(py, inner).expect("Unable to allocate cell");

            Outer {
                inner
            }
        }
    }
//     #[expose_struct("opaque")]
//     struct ImplMyTrait {
//         inner: super::ImplMyTrait,
//     }
//     impl ImplMyTrait {
//         fn into_inner(self) -> super::ImplMyTrait {
//             self.inner
//         }
//     }
//     #[expose_fn]
//     fn impl_my_trait_new(val: u32) -> MyTraitStruct {
//         super::ImplMyTrait(val).into_trait_struct()
//     }
//
//     #[expose_trait]
//     pub trait MyTrait: super::MyTrait {
//         #[expose_trait(original = "method")]
//         fn _wrapper_method(&self, s: String) -> String;
//     }
//     impl super::MyTrait for MyTraitStruct {
//         fn method(&self, s: String) -> String {
//             self._wrapper_method(s)
//         }
//     }
//
//     #[expose_fn]
//     fn use_trait(t: &MyTraitStruct) {
//         use super::MyTrait;
//
//         let ret = t.method("Hello from Rust".to_string());
//         println!("Returned: {}", ret);
//     }
}
