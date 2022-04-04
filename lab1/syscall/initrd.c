#include <sys/syscall.h>      /* Definition of SYS_* constants */
#include <unistd.h>
#include <stdio.h>

int main()  {
	char buf[16];
	buf[15] = '\0';
	int ret_val;

	// test0, with buf_len == 13
	ret_val = syscall(548, buf, 13);
	printf("size: %d, ret_val: %d, buf: %s\n", 13, ret_val, buf);

	// test1, with buf_len == 14
	ret_val = syscall(548, buf, 14);
	printf("size: %d, ret_val: %d, buf: %s\n", 14, ret_val, buf);

	// test2, with buf_len == 13
	ret_val = syscall(548, buf, 13);
	printf("size: %d, ret_val: %d, buf: %s\n", 13, ret_val, buf);
	
	printf("test finished\n");
	return 0;
}
