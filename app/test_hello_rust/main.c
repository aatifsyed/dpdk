#include <stddef.h>
#include <stdio.h>

size_t len(const char *);
char *concat(const char *, const char *);
void free_buf(char *);

int main(int argc, const char **argv)
{
    size_t my_len = len("hello");
    if (printf("%ld\n", my_len) == EOF)
    {
        return 1;
    }
    char *hello_world = concat("hello", "world");
    if (hello_world != NULL)
    {
        if (printf("%s\n", hello_world) == EOF)
        {
            free_buf(hello_world);
            return 1;
        }
        free_buf(hello_world);
    }
    else
    {
        printf("failed to concat\n");
    }
    return 0;
}
