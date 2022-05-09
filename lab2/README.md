# Lab2 README

刘良宇 PB20000180

## SHELL

### 编译

```bash
cargo build --release
```

版本：`rustc 1.62.0-nightly`

### 选做内容

- `$` 开头会直接作为环境变量被替换,例如可以 `echo $HOME`
- 支持处理 ctrl + D 退出 shell
- 对于 history 记录持久保持在 `~/.llysh_history`
- 支持对于 `~` 开头的路径参数的识别，并优化 `cd` 的默认行为，只输入 `cd` 会跳转至 home 目录

### 说明

内建命令统一采用函数调用的方式执行，因此无法对 `history` 等命令重定向输出（但不会影响其他重定向和管道其他部分的执行）

## Strace

`make` 即可
