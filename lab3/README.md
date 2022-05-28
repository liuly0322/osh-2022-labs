# 实验三

## 编译

`rustc 1.62.0-nightly`

`cargo build --release`

## 文件夹结构

- dual_chat_room 双人聊天室
- group_chatroom_multi_thread 多线程多人聊天室， `mpsc` 无锁队列
- group_chatroom_epoll 多人聊天室的 `epoll` I/O 复用实现
