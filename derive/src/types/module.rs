use syn::Ident;

#[derive(Debug)]
pub enum ModuleItem {
    Function(Ident),
    Structure(Ident),
    Module(Ident),
}
