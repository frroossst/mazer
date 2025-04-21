#! /bin/bash

rustup toolchain install nightly
rustup component add rustc-codegen-cranelift-preview --toolchain nightly
rustup default nightly

