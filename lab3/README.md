# 实验三

## 编译

`rustc 1.62.0-nightly`

`cargo build --release`

## 文件夹结构

- dual_chat_room 双人聊天室
- group_chatroom_multi_thread 多线程多人聊天室， `mpsc` 无锁队列
- group_chatroom_epoll 多人聊天室的 `epoll` I/O 复用实现
- group_chatroom_async 多人聊天室的异步协程实现，采用 `tokio` 库
- group_chatroom_io_uring 多人聊天室的 `io_uring` 实现

## 说明

- 换行为分割符：recv 后遍历接受缓冲区
- send 无法一次发送所有数据：采用 rust 的 write_all 方法

