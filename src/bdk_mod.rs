use super::*;

#[derive(Debug)]
pub enum BDKError {
    Any(Box<dyn std::error::Error>),
}
impl From<::bdk::bitcoin::Error> for BDKError {
    fn from(err: ::bdk::bitcoin::Error) -> Self {
        BDKError::Any(err.into())
    }
}
#[cfg(feature = "python")]
impl_py_error!(BDKError);
#[cfg(feature = "c")]
impl langs::IntoPlatformError for BDKError {
    type TargetType = i32;

    fn into_platform_error(self) -> Self::TargetType {
        // match self {
        // }
        -1
    }

    fn ok() -> Self::TargetType {
        0
    }
}

#[expose_mod]
mod bdk {
    use std::ops::Deref;

    use super::BDKError;
    use crate::bitcoin_mod::bitcoin as bitcoin_mod;

    #[expose_struct("opaque")]
    pub struct KeychainKind {
        inner: bdk::KeychainKind,
    }
    wrap_struct!(KeychainKind, bdk::KeychainKind);
    #[expose_impl]
    impl KeychainKind {
        #[destructor]
        fn destroy(_s: Self) {}

        fn external() -> Self {
            bdk::KeychainKind::External.into()
        }
        fn internal() -> Self {
            bdk::KeychainKind::Internal.into()
        }
        fn is_internal(&self) -> bool {
            self.deref() == &bdk::KeychainKind::Internal
        }
        fn is_external(&self) -> bool {
            self.deref() == &bdk::KeychainKind::External
        }
    }

    #[expose_struct("opaque")]
    pub struct FeeRate {
        inner: bdk::FeeRate,
    }
    wrap_struct!(FeeRate, bdk::FeeRate);
    #[expose_impl]
    impl FeeRate {
        #[destructor]
        fn destroy(_s: Self) {}

        fn from_btc_per_kvb(btc_per_kvb: f32) -> Self {
            bdk::FeeRate::from_btc_per_kvb(btc_per_kvb).into()
        }
        fn from_sat_per_vb(sat_per_vb: f32) -> Self {
            bdk::FeeRate::from_sat_per_vb(sat_per_vb).into()
        }
        fn default_min_relay_fee() -> Self {
            bdk::FeeRate::default_min_relay_fee().into()
        }
        fn as_sat_vb(&self) -> f32 {
            self.deref().as_sat_vb()
        }
    }

    #[expose_struct("opaque")]
    pub struct LocalUtxo {
        inner: bdk::LocalUtxo,
    }
    wrap_struct!(LocalUtxo, bdk::LocalUtxo);
    #[expose_impl]
    impl LocalUtxo {
        #[constructor]
        fn new(
            outpoint: &bitcoin_mod::OutPoint,
            txout: &bitcoin_mod::TxOut,
            keychain: &KeychainKind,
        ) -> Self {
            bdk::LocalUtxo {
                outpoint: outpoint.deref().clone().into(),
                txout: txout.deref().clone().into(),
                keychain: keychain.deref().clone().into(),
            }
            .into()
        }
        #[destructor]
        fn destroy(_s: Self) {}

        // TODO: getter/setter
    }

    #[expose_struct("opaque")]
    pub struct ForeignUtxo {
        outpoint: bitcoin_mod::OutPoint,
    }
}
