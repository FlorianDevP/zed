#!/usr/bin/env just --justfile

set windows-shell := ["busybox", "sh", "-eu", "-c"]

profile := "release"
target := arch() + "-unknown-linux-musl"
src_path := "." / "target" / target / profile / "remote_server"
RUSTFLAGS := env('RUSTFLAGS', "") + "-C target-feature=+crt-static"
zigbuild_arg := if profile == "debug" { "--features debug-embed" } else { "" }

# destination

release_channel := "dev"
version_str := "build"
binary_name := "zed-remote-server-" + release_channel + "-" + version_str
dst_path := "~" / ".zed_server" / binary_name

default: build

build: remote_server build_editor

build_editor:
    cargo build --profile {{ profile }}
    #--features remote/build-remote-server-binary

remote_server: build_remote_server wsl_deploy_remote

build_remote_server:
    export RUSTFLAGS="{{ RUSTFLAGS }}" && \
    cargo zigbuild --package remote_server {{ zigbuild_arg }} --profile {{ profile }} --target {{ target }}

wsl_deploy_remote:
    wsl --distribution Ubuntu --shell-type standard -- cp -fT "{{ src_path }}" "{{ dst_path }}"
