CC           := gcc
CFLAGS       := -Og -Wall -Wextra
LIB_PATH     := `pwd`/../target/debug

RUST_SRCS    := $(shell find ../src ../derive -type f -name "*.rs")
CARGO_TOML   := $(shell find ../ ../derive -type f -name "Cargo.toml")

all: example

../target/debug/libbdk_ffi.so ./bindings.h: $(RUST_SRCS) $(CARGO_TOML)
	cargo build --features c
	# Update timestamps to avoid rebuilding every single time
	touch ../target/debug/libbdk_ffi.so
	touch ./bindings.h

example: example.c ../target/debug/libbdk_ffi.so ./bindings.h
	$(CC) $(CFLAGS) -L$(LIB_PATH) -lbdk_ffi -Wl,-rpath,$(LIB_PATH) example.c -o example

run: example
	./example

.PHONY: clean
clean:
	rm ./example
