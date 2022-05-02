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
- 优化 `cd` 的默认行为
  - 输入 `cd` 会跳转至 home 目录（只要没有手动 unset $HOME）
  - 允许输入路径以 `~` 开头
- 优化 shell 显示。支持绿色显示当前路径，并且如果当前目录在 `home` 下，前缀会被替换为 `~`

### 说明

出于统一性考虑，内建命令统一采用函数调用的方式执行，因此无法对 `history` 重定向或者使其出现在管道

~~可以自行使用 `~/.llysh_history` 获取想要的信息~~

## Strace
