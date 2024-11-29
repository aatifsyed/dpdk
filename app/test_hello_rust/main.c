#include <stddef.h>
#include <stdio.h>
#include <../lib/kvargs/rte_kvargs.h>
#include <assert.h>

int print_each_arg(const char *key, const char *val, void *opaque)
{
    int rc;
    if (val != NULL)
        rc = printf("%s => %s\n", key, val);
    else
        rc = printf("%s\n", key);
    if (rc == EOF)
        return -1;
    return 0;
}

int main(int argc, const char **argv)
{
    for (int i = 0; i < argc; i++)
    {
        if (i == 0)
            continue;
        struct rte_kvargs *kvlist = rte_kvargs_parse(argv[i], NULL);
        if (kvlist == NULL)
        {
            printf("failed to parse %s\n", argv[i]);
            return 1;
        }
        else
        {
            rte_kvargs_process_opt(kvlist, NULL, print_each_arg, NULL);
            rte_kvargs_free(kvlist);
        }
    }

    return 0;
}
