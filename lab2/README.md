# Lab2 README

刘良宇 PB20000180

## SHELL

### 编译

```bash
cargo build --release
```

版本：`rustc 1.62.0-nightly`

### 必做内容说明

运行 shell 后会自动产生 `~/.llysh_history`

~~可以跑完清理一下~~

```bash
rm ~/.llysh_history
```

### 选做内容

- 替换环境变量。利用迭代器遍历 `args` ，`$` 开头会直接作为环境变量被替换
  - `echo` 默认使用 `/bin/echo`, shell 不要求 `echo` 内建
- 支持处理 ctrl + D 退出 shell
- 优化 `cd` 的默认行为
  - 输入 `cd` 会跳转至 home 目录（只要没有手动 unset $HOME）
  - 输入 `cd -` 会跳转至上一次进入的目录
  - 允许输入路径以 `~` 开头
- 优化 shell 显示。支持绿色显示当前路径，并且如果当前目录在 `home` 下，前缀会被替换为 `~`
- 支持显示当进程退出（但没有正常退出时）显示退出代码，例如：
  ```shell
  ~/osh-2022/lab2/shell> test
  [1]~/osh-2022/lab2/shell>
  ```
  显示退出代码 1

## Strace
