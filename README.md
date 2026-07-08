# Forge-1

A local, free AI coding agent — runs entirely on your own machine via Ollama, exposed as an API so it can plug into other tools with an API key.

## What it does

Forge-1 is a small autonomous coding agent: give it a task, and it reasons step by step, deciding whether to read a file, write a file, or run a shell command, until the task is done. Everything runs locally — no cloud costs, no data leaving your machine.

## Requirements

- [Rust](https://rustup.rs) (to build it)
- [Ollama](https://ollama.com) (to run the AI model locally)

## Setup

1. Install Ollama, then pull the coding models: