#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <sys/stat.h>

int main() {
    char input[20] = {0};
    char flag[40] = {0};

    puts("hello from notsh v1.0");
    printf("would you like a flag? ");

    fgets(input, 20, stdin);

    input[strcspn(input, "\n")] = 0;

    if (strcmp(input, "yes") == 0) {
        puts("ok!");

        int fd = open("./flag", O_RDONLY);
        read(fd, flag, 40);
        write(1, flag, 40);
    } else if (strcmp(input, "shell") == 0) {
        system("/bin/sh");
    } else {
        puts("better luck next time!");
    }

    return 0;
}