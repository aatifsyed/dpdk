#include <stddef.h>
#include <stdio.h>

size_t len(const char *);

int main(int argc, const char **argv)
{
    size_t my_len = len("hello");
    if (printf("%ld", my_len) == EOF)
    {
        return 1;
    }
    return 0;
}
