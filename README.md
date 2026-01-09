<div align = "center">
<img src="logo.svg" width="200">

# The Moss Programming Language

-- _Frontend for all DSL_ --

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg) ![Contributions](https://img.shields.io/badge/contributions-welcome-brightgreen) ![GPU](https://img.shields.io/badge/domain-GPU%20Computing-yellowgreen) ![status](https://img.shields.io/badge/status-prototype-red)

</div>

## Start Developing
0. setup [Rust](https://rust-lang.org/learn/get-started/) developing environment, download [Zed](https://zed.dev/download), have [Python]() in PATH.
1. fork and recursively clone this repo.

    ```
    git clone https://github.com/<your name>/moss-lang --recursive
    ```
2. build the project.
    ```
    cargo build
    ```
3. run the [install script](install.py).
    ```
    python install.py
    ```
    > you can uninstall by run it with one arbitrary argument
    > ```
    > python install.py u
    > ```
    try run `moss` in any terminal to check if installed successfully.
    ```
    moss
    ```
    got:
    ```
    Moss Lang v0.1.0
    ```
4. run Zed from terminal.
    ```
    zed --foreground
    ```
5. install [zed extension](zed-extension) in Zed via `Extensins/Install Dev Extension`.
6. restart Zed, open [example Moss project](language_example/hallo_world).
   

## Why Moss?

In this AI era, countless GPU computing DSL (domain specific lanuage) has came into been. However, most of them use python or C++ as frontend, which is hard to tailerd for DSL usage.

Moss lang aims to provide a mordern language frontend which is easy to JIT (generate code during execution) or AOT (generate code before execution) any DSL code with corresponding integration.
