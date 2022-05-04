#include <sys/ptrace.h>
#include <sys/user.h>
#include <sys/wait.h>
#include <syscall.h>
#include <unistd.h>
#include <cstdlib>
#include <iostream>

int main(int argc, char** argv) {
    pid_t pid = fork();
    switch (pid) {
        case -1:
            exit(1);
        case 0:
            ptrace(PTRACE_TRACEME, 0, 0, 0);
            execvp(argv[1], argv + 1);
            exit(1);
    }

    // parent
    waitpid(pid, 0, 0);
    ptrace(PTRACE_SETOPTIONS, pid, 0, PTRACE_O_EXITKILL);

    while (true) {
        // failed to enter
        if (ptrace(PTRACE_SYSCALL, pid, 0, 0) == -1) {
            exit(0);
        }
        waitpid(pid, 0, 0);

        struct user_regs_struct regs;
        ptrace(PTRACE_GETREGS, pid, 0, &regs);
        long syscall = regs.orig_rax;
        fprintf(stderr, "%ld(%ld, %ld, %ld, %ld, %ld, %ld)\n", syscall,
                (long)regs.rdi, (long)regs.rsi, (long)regs.rdx, 
                (long)regs.r10, (long)regs.r8, (long)regs.r9);

        // stop on the exit
        ptrace(PTRACE_SYSCALL, pid, 0, 0);
        waitpid(pid, 0, 0);
    }
}