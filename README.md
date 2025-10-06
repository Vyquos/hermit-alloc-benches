# hermit-alloc-benches

A simple benchmarking suite for generating statistics about allocator performance.

The benchmarks are intended to run on the [Hermit Operating System][hermit],
testing the [`virtual_alloc`][virtual-alloc] allocator. This crate only contains
benchmarking files. The benchmarks are performed either manually or using
[criterion.rs][criterion], leveraging [rayon] to test with high concurrency.

[criterion]: https://github.com/bheisler/criterion.rs
[hermit]: https://hermit-os.org/
[rayon]: https://github.com/rayon-rs/rayon
[virtual-alloc]: https://github.com/Vyquos/virtual-alloc-hermit

## Dependencies

A nightly [Rust toolchain][rust-toolchain] is required to compile and run
hermit-alloc-benches. The default toolchain for the project is set by
[`rust-toolchain.toml`][toolchain-file]. The current (per-directory) toolchain
can be inspected using [rustup] by running `rustup show active-toolchain -v` in
the repository root (if the toolchain isn't found, rustup will install it
first).

Hermit uses a custom build of the Rust Standard Library (rust std). To build
virtual\_alloc for the `x86_64-unknown-hermit` target, the std must be installed
for the corresponding Rust toolchain (see above). Installation instructions can
be found in the [rust-std-hermit] repository (adapted for nightly toolchains).

[rust-std-hermit]: https://github.com/Vyquos/rust-std-hermit.git
[rust-toolchain]: https://ehuss.github.io/rustup/concepts/toolchains.html
[rustup]: https://rustup.rs/
[toolchain-file]: ./rust-toolchain.toml

## Running Hermit

To run the benchmarks in Hermit, the use of the [QEMU emulator][qemu] with the
[hermit loader][hermit-loader] is recommended.

The QEMU emulator for x86\_64 (the AMD / Intel 64 bit architecture) devices can
be installed on most Linux distributions by running
```sh
sudo apt-get install qemu-system-x86
```
(or the respective equivalent for your package manager of choice). For
instructions on how to download and build the hermit loader for QEMU, check the
[repo's README file][hermit-loader-readme].

Benchmark results are saved as files on your local system. To allow Hermit to
write these results, a directory must be mounted using [virtio-fs]. This
requires [virtiofsd] to be running. After installing and compiling virtiofsd,
create and run a new virtio-fs directory by executing the following (in a
separate shell from QEMU):
```sh
mkdir bench-results
chmod 777 bench-results
sudo /path/to/virtiofsd/target/release/virtiofsd --socket-path=/tmp/vhostfs.sock --shared-dir="$(realpath bench-results/)"
```
Then, ensure that QEMU can access the socket:
```sh
sudo chmod 777 /tmp/vhostfs.sock
```
and run the above QEMU command. Statistics will be generated under the
"bench-results" directory.

[hermit-loader-readme]: https://github.com/hermit-os/loader/blob/main/README.md
[hermit-loader]: https://github.com/hermit-os/loader.git
[qemu-options]: https://www.qemu.org/docs/master/system/qemu-manpage.html
[qemu]: https://www.qemu.org/download/#linux
[virtio-fs]: https://virtio-fs.gitlab.io/
[virtiofsd]: https://gitlab.com/virtio-fs/virtiofsd

## criterion.rs Benchmarks

To compile the criterion benchmarks for use inside Hermit, run
```sh
cargo bench --no-run
```
or, to benchmark `virtual_alloc`:
```sh
cargo bench --no-run --features virtual_alloc
```

Pass the path to the generated benchmark executable ("target/x86\_64-unknown-hermit/release/deps/rayon\_alloc-...") to QEMU:
```sh
qemu-system-x86_64 \
    -enable-kvm \
    -cpu host \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -display none -serial stdio \
    -chardev socket,id=char0,path=/tmp/vhostfs.sock \
    -device vhost-user-fs-pci,queue-size=1024,chardev=char0,tag=root \
    -object memory-backend-file,id=mem,size=10G,mem-path=/dev/shm,share=on \
    -numa node,memdev=mem \
    -kernel LOADER_PATH \
    -initrd target/x86_64-unknown-hermit/release/deps/rayon_alloc-... \
    -smp 2 \
    -m 10G \
    -append "-- --bench"
```

## Manual Benchmarks

The benchmarks can also be run manually. To do so, perform the steps to run
Hermit as above, then compile the manual benchmarks with:
```sh
cargo build --release --bin alloc_benches
```

The benchmarks can be run using (after starting virtiofsd):
```sh
qemu-system-x86_64 \
    -enable-kvm \
    -cpu host \
    -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
    -display none -serial stdio \
    -chardev socket,id=char0,path=/tmp/vhostfs.sock \
    -device vhost-user-fs-pci,queue-size=1024,chardev=char0,tag=root \
    -object memory-backend-file,id=mem,size=10G,mem-path=/dev/shm,share=on \
    -numa node,memdev=mem \
    -kernel LOADER_PATH \
    -initrd target/x86_64-unknown-hermit/release/alloc_benches \
    -smp 2 \
    -m 10G \
    -append "-- OPTIONS"
```

Replace `OPTIONS` with `--help` to print available options.
