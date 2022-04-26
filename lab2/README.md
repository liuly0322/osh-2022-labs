# Lab2 README

刘良宇 PB20000180

## SHELL

### 编译

```bash
cargo build --release
```

版本：`rustc 1.62.0-nightly`

### 选做内容

- 替换环境变量。利用迭代器遍历 `args` ，`$` 开头会直接作为环境变量被替换
  - `echo` 默认使用 `/bin/echo`, shell 不要求 `echo` 内建
- 支持处理 ctrl + D 退出 shell
- 优化 `cd` 的默认行为，在 `$HOME` 环境变量被设置的前提下，输入 `cd` 会跳转至 home 目录
- 优化 shell 显示。支持绿色显示当前路径，并且如果当前目录在 `home` 下，会被替换为 `~`

## Strace
