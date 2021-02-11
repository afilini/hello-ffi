from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="hello",
    version="1.0",
    rust_extensions=[RustExtension("hello.hello", path="../Cargo.toml", features=["python"], binding=Binding.PyO3)],
    packages=["hello"],
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
