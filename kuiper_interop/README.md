# Kuiper interop

The Kuiper interop package provides a Foreign Function Interface for the Kuiper library following the C ABI. It allows
users to compile and execute Kuiper functions from C, as well as other languages that can follow the same binary
interface. The Kuiper interop package forms the basis for other language bindings, such as for [.NET](../KuiperNet/)

The C interface itself is documented through the [`kuiper.h`](./kuiper.h) header file, which you can also include if
you need to call Kuiper from C.

To test the interrop library, run the `compile_and_run.sh` script in the `test` folder. This will compile a small C
program which uses the interop library to perform a small computation.
