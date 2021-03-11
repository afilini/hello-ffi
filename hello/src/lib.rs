/// Structure documentation
///
/// Multiple lines
#[derive(Debug)]
pub struct HelloStruct {
    pub init: String,
}

impl HelloStruct {
    /// Static method docs
    pub fn hello_static(string: &str) -> String {
        format!("Hello Static {}", string)
    }

    /// Method documentation
    pub fn hello_method(&self, string: &str) -> String {
        format!("Hello Method. Init({}): {}", self.init, string)
    }
}

#[derive(Debug)]
pub struct Wallet {
    wallet_name: String,
}

impl Wallet {
    pub fn new(wallet_name: &str) -> Self {
        Wallet {
            wallet_name: wallet_name.to_string(),
        }
    }

    pub fn create_tx(&self) -> TxBuilder<'_, DoubleCS> {
        TxBuilder::new(self, DoubleCS(0))
    }
}

pub trait CoinSelectionAlgorithm: std::fmt::Debug {
    fn do_something(&self, val: u32) -> u32;
}

#[derive(Debug)]
pub struct DoubleCS(u32);

impl DoubleCS {
    pub fn new(init: u32) -> Self {
        DoubleCS(init)
    }
}

impl CoinSelectionAlgorithm for DoubleCS {
    fn do_something(&self, val: u32) -> u32 {
        self.0 + val * 2
    }
}

#[derive(Debug)]
pub struct TripleCS(u32);

impl TripleCS {
    pub fn new(init: u32) -> Self {
        dbg!(init);
        TripleCS(init)
    }
}

impl CoinSelectionAlgorithm for TripleCS {
    fn do_something(&self, val: u32) -> u32 {
        dbg!(self, val);
        self.0 + val * 3
    }
}

#[derive(Debug)]
pub struct TxBuilder<'w, C: CoinSelectionAlgorithm> {
    pub wallet: &'w Wallet,
    pub flag: bool,
    pub cs: C,
}

impl<'w, C: CoinSelectionAlgorithm> TxBuilder<'w, C> {
    fn new(wallet: &'w Wallet, cs: C) -> Self {
        TxBuilder {
            wallet,
            cs,
            flag: false,
        }
    }

    pub fn enable_flag(&mut self) -> &mut Self {
        self.flag = true;
        self
    }

    pub fn disable_flag(&mut self) -> &mut Self {
        self.flag = false;
        self
    }

    pub fn coin_selection<N: CoinSelectionAlgorithm>(self, cs: N) -> TxBuilder<'w, N> {
        TxBuilder {
            wallet: self.wallet,
            flag: self.flag,
            cs,
        }
    }

    pub fn finish(self) -> u32 {
        dbg!(self.flag);
        self.cs.do_something(5)
    }

    pub fn get_wallet_name(&self) -> String {
        self.wallet.wallet_name.clone()
    }

    #[doc(hidden)]
    pub fn convert_internal_cs<N: CoinSelectionAlgorithm, F: Fn(C) -> N>(
        self,
        f: F,
    ) -> TxBuilder<'w, N> {
        let cs = f(self.cs);

        TxBuilder {
            wallet: self.wallet,
            flag: self.flag,
            cs,
        }
    }

    #[doc(hidden)]
    pub fn mut_cs(&mut self) -> &mut C {
        &mut self.cs
    }
}
