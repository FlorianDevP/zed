#!/usr/bin/env just --justfile

set windows-shell := ["busybox", "sh", "-eu", "-c"]

workflow_path := '.github/workflows/fork_sync_with_Remote.yml'
ubuntu_slim := 'ubuntu-slim=ghcr.io/catthehacker/ubuntu:act-latest'

default: build_workflows

build_workflows:
    cargo xtask workflows

dispatch_fork_sync_workflow: build_workflows
    gh act workflow_dispatch -W {{ workflow_path }} -s GITHUB_TOKEN="$(gh auth token)" -P {{ ubuntu_slim }}
