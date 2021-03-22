use super::*;

#[expose_mod]
mod bitcoin {
    use super::BitcoinError;
    use std::ops::Deref;

    #[expose_struct("opaque")]
    pub struct Script {
        inner: bdk::bitcoin::Script,
    }
    wrap_struct!(Script, bdk::bitcoin::Script);

    #[expose_impl]
    impl Script {
        #[constructor]
        fn new(script: Vec<u8>) -> Self {
            bdk::bitcoin::Script::from(script).into()
        }
        #[destructor]
        fn destroy(_s: Self) {}

        fn empty() -> Self {
            bdk::bitcoin::Script::new().into()
        }
        fn from_hex(hex: String) -> Result<Self, BitcoinError> {
            use bdk::bitcoin::hashes::hex::FromHex;
            Ok(bdk::bitcoin::Script::from_hex(&hex)?.into())
        }
        fn to_hex(&self) -> String {
            use bdk::bitcoin::hashes::hex::ToHex;
            self.deref().to_hex()
        }

        fn asm(&self) -> String {
            self.deref().asm()
        }
    }

    #[expose_struct("opaque")]
    pub struct Network {
        inner: bdk::bitcoin::Network,
    }
    wrap_struct!(Network, bdk::bitcoin::Network);
    #[expose_impl]
    impl Network {
        #[constructor]
        fn from_string(s: String) -> Result<Self, BitcoinError> {
            use std::str::FromStr;

            Ok(bdk::bitcoin::Network::from_str(&s)?.into())
        }
        #[destructor]
        fn destroy(_s: Self) {}

        fn bitcoin() -> Self {
            bdk::bitcoin::Network::Bitcoin.into()
        }
        fn testnet() -> Self {
            bdk::bitcoin::Network::Testnet.into()
        }

        fn to_string(&self) -> String {
            self.deref().to_string()
        }
    }

    #[expose_struct("opaque")]
    pub struct Address {
        inner: bdk::bitcoin::Address,
    }
    wrap_struct!(Address, bdk::bitcoin::Address);
    #[expose_impl]
    impl Address {
        fn from_script(script: &Script, network: &Network) -> Option<Self> {
            bdk::bitcoin::Address::from_script(script.deref(), network.deref().clone())
                .map(Into::into)
        }

        #[constructor]
        fn from_string(s: String) -> Result<Self, BitcoinError> {
            use std::str::FromStr;

            Ok(bdk::bitcoin::Address::from_str(&s)?.into())
        }
        #[destructor]
        fn destroy(_s: Self) {}

        fn to_string(&self) -> String {
            self.deref().to_string()
        }

        #[getter]
        fn get_script(&self) -> Script {
            self.script_pubkey().into()
        }

        fn network(&self) -> Network {
            self.inner.network.into()
        }
    }

    pub use transaction::*;
    #[expose_mod]
    mod transaction {
        use super::*;
        use std::ops::Deref;

        #[expose_struct("opaque")]
        pub struct OutPoint {
            inner: bdk::bitcoin::OutPoint,
        }
        wrap_struct!(OutPoint, bdk::bitcoin::OutPoint);
        #[expose_impl]
        impl OutPoint {
            #[constructor]
            fn from_string(s: String) -> Result<Self, BitcoinError> {
                use std::str::FromStr;

                Ok(bdk::bitcoin::OutPoint::from_str(&s)?.into())
            }
            #[destructor]
            fn destroy(_s: Self) {}

            fn new(txid: [u8; 32], vout: u32) -> Self {
                bdk::bitcoin::OutPoint {
                    txid: bdk::bitcoin::Txid::from_hash(bdk::bitcoin::hashes::Hash::from_inner(
                        txid,
                    )),
                    vout,
                }
                .into()
            }
            fn get_txid(&self) -> &[u8] {
                &self.txid
            }
            fn get_vout(&self) -> u32 {
                self.vout
            }
            fn set_txid(&mut self, txid: [u8; 32]) {
                self.txid =
                    bdk::bitcoin::Txid::from_hash(bdk::bitcoin::hashes::Hash::from_inner(txid));
            }
            fn set_vout(&mut self, vout: u32) {
                self.vout = vout;
            }

            fn to_string(&self) -> String {
                self.deref().to_string()
            }
        }

        #[expose_struct("opaque")]
        pub struct TxOut {
            inner: bdk::bitcoin::TxOut,
        }
        wrap_struct!(TxOut, bdk::bitcoin::TxOut);
        #[expose_impl]
        impl TxOut {
            #[constructor]
            fn new(script_pubkey: &Script, value: u64) -> Self {
                bdk::bitcoin::TxOut {
                    script_pubkey: script_pubkey.deref().clone().into(),
                    value,
                }
                .into()
            }
            #[destructor]
            fn destroy(_s: Self) {}

            fn get_script_pubkey(&self) -> Script {
                self.script_pubkey.clone().into()
            }
            fn get_value(&self) -> u64 {
                self.value
            }
            fn set_script_pubkey(&mut self, script_pubkey: &Script) {
                self.script_pubkey = script_pubkey.deref().clone().into();
            }
            fn set_value(&mut self, value: u64) {
                self.value = value;
            }
        }

        #[expose_struct("opaque")]
        pub struct TxIn {
            inner: bdk::bitcoin::TxIn,
        }
        wrap_struct!(TxIn, bdk::bitcoin::TxIn);
        #[expose_impl]
        impl TxIn {
            #[constructor]
            fn new(
                previous_output: &OutPoint,
                script_sig: &Script,
                sequence: u32,
                witness: Vec<Vec<u8>>,
            ) -> Self {
                bdk::bitcoin::TxIn {
                    previous_output: previous_output.deref().clone().into(),
                    script_sig: script_sig.deref().clone().into(),
                    sequence,
                    witness,
                }
                .into()
            }
            #[destructor]
            fn destroy(_s: Self) {}
        }

        #[expose_struct("opaque")]
        pub struct Transaction {
            inner: bdk::bitcoin::Transaction,
        }
        wrap_struct!(Transaction, bdk::bitcoin::Transaction);
        #[expose_impl]
        impl Transaction {
            #[constructor]
            fn new(version: i32, lock_time: u32, input: Vec<&TxIn>, output: Vec<&TxOut>) -> Self {
                bdk::bitcoin::Transaction {
                    version,
                    lock_time,
                    input: input.into_iter().map(|i| i.deref().clone()).collect(),
                    output: output.into_iter().map(|o| o.deref().clone()).collect(),
                }
                .into()
            }
            #[destructor]
            fn destroy(_s: Self) {}

            fn from_hex(hex: String) -> Result<Self, BitcoinError> {
                use bdk::bitcoin::consensus::deserialize;
                use bdk::bitcoin::hashes::hex::FromHex;

                let bytes = Vec::<u8>::from_hex(&hex)?;
                let deserialized: bdk::bitcoin::Transaction = deserialize(&bytes)?;
                Ok(deserialized.into())
            }
            fn to_hex(&self) -> String {
                use bdk::bitcoin::consensus::encode::serialize_hex;

                serialize_hex(self.deref())
            }
        }
    }
}

#[derive(Debug)]
pub enum BitcoinError {
    Bitcoin(bdk::bitcoin::Error),
    ParseOutPoint(bdk::bitcoin::blockdata::transaction::ParseOutPointError),
    BitcoinEncode(bdk::bitcoin::consensus::encode::Error),

    Hex(bdk::bitcoin::hashes::hex::Error),
    Address(bdk::bitcoin::util::address::Error),
    IO(std::io::Error),
}
impl From<bdk::bitcoin::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::Error) -> Self {
        BitcoinError::Bitcoin(e)
    }
}
impl From<bdk::bitcoin::blockdata::transaction::ParseOutPointError> for BitcoinError {
    fn from(e: bdk::bitcoin::blockdata::transaction::ParseOutPointError) -> Self {
        BitcoinError::ParseOutPoint(e)
    }
}
impl From<bdk::bitcoin::consensus::encode::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::consensus::encode::Error) -> Self {
        BitcoinError::BitcoinEncode(e)
    }
}
impl From<bdk::bitcoin::hashes::hex::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::hashes::hex::Error) -> Self {
        BitcoinError::Hex(e)
    }
}
impl From<bdk::bitcoin::util::address::Error> for BitcoinError {
    fn from(e: bdk::bitcoin::util::address::Error) -> Self {
        BitcoinError::Address(e)
    }
}
impl From<std::io::Error> for BitcoinError {
    fn from(e: std::io::Error) -> Self {
        BitcoinError::IO(e)
    }
}
#[cfg(feature = "python")]
impl_py_error!(BitcoinError);
#[cfg(feature = "c")]
impl langs::IntoPlatformError for BitcoinError {
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
