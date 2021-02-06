CC=gcc
CFLAGS=-Og -Wall -Wextra
LIB_PATH=`pwd`/target/debug

all: example

./target/debug/libhello_ffi.so ./bindings.h:
	cargo build

example: example.c ./target/debug/libhello_ffi.so ./bindings.h
	${CC} ${CFLAGS} -L${LIB_PATH} -lhello_ffi -Wl,-rpath,${LIB_PATH} example.c -o example

run: example
	./example

.PHONY: clean
clean:
	rm ./example
