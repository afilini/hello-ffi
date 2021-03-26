#include <assert.h>
#include <stdio.h>

#include "bindings.h"

int main() {
    Inner i = { .val = 10 };

    Outer *o = NULL;
    outer_new(&i, &o);

    printf("%u\n", outer_get_inner(o)->val);
    outer_get_inner(o)->val *= 5;
    printf("%u\n", outer_get_inner(o)->val);

    Inner i2 = { .val = 1000 };
    outer_set_inner(o, &i2);

    printf("%u\n", outer_get_inner(o)->val);

    outer_destroy(o);
}
