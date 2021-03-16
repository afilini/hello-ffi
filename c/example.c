#include <assert.h>
#include <stdio.h>

#include "bindings.h"

int main() {
    Script *s = NULL;
    int ret = script_from_hex("a91457d6b4ded38193013643b03b4472e15f80bc465787", &s);
    assert(ret == 0);

    Network *n = NULL;
    network_testnet(&n);

    Address *a = NULL;
    address_from_script(s, n, &a);
    assert(a != NULL);

    char *address_str = address_to_string(a);
    printf("Address: %s\n", address_str);
    free(address_str);

    char *script_hex = script_to_hex(s);
    printf("Script: %s\n", script_hex);
    free(script_hex);

    script_destroy(s);
    network_destroy(n);
    address_destroy(a);
}
