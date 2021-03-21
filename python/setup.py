from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="bdk",
    version="1.0",
    rust_extensions=[RustExtension("bdk.test_mod", path="../Cargo.toml", features=["python"], binding=Binding.PyO3)],
    packages=["bdk"],
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
