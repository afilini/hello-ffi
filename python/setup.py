from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="bdk",
    version="0.5.0",
    rust_extensions=[
        RustExtension("bdk.bitcoin", path="../Cargo.toml", features=["python"], binding=Binding.PyO3),
        RustExtension("bdk.bdk", path="../Cargo.toml", features=["python"], binding=Binding.PyO3),
    ],
    packages=["bdk"],
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
