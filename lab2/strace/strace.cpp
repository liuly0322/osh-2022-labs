#define _POSIX_C_SOURCE 200112L

/* C standard library */
#include <errno.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* POSIX */
#include <sys/user.h>
#include <sys/wait.h>
#include <unistd.h>

/* Linux */
#include <sys/ptrace.h>
#include <syscall.h>

int main(int argc, char** argv) {
    pid_t pid = fork();
    switch (pid) {
        case -1: /* error */
            exit(1);
        case 0: /* child */
            ptrace(PTRACE_TRACEME, 0, 0, 0);
            /* Because we're now a tracee, execvp will block until the parent
             * attaches and allows us to continue. */
            execvp(argv[1], argv + 1);
            exit(1);
    }

    /* parent */
    waitpid(pid, 0, 0);  // sync with execvp
    ptrace(PTRACE_SETOPTIONS, pid, 0, PTRACE_O_EXITKILL);

    while (true) {
        /* Enter next system call */
        ptrace(PTRACE_SYSCALL, pid, 0, 0);
        waitpid(pid, 0, 0);
        /* Gather system call arguments */
        struct user_regs_struct regs;
        ptrace(PTRACE_GETREGS, pid, 0, &regs);

        long syscall = regs.orig_rax;

        /* Print a representation of the system call */
        fprintf(stderr, "%ld(%ld, %ld, %ld, %ld, %ld, %ld)\n", syscall,
                (long)regs.rdi, (long)regs.rsi, (long)regs.rdx, (long)regs.r10,
                (long)regs.r8, (long)regs.r9);

        /* Run system call and stop on exit */
        ptrace(PTRACE_SYSCALL, pid, 0, 0);
        waitpid(pid, 0, 0);

        /* Get system call result */
        if (ptrace(PTRACE_GETREGS, pid, 0, &regs) == -1) {
            exit(regs.rdi);  // system call was _exit(2) or similar
        }
    }
}