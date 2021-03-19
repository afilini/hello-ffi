#include <assert.h>
#include <stdio.h>

#include "bindings.h"

static void trait_destroy(__attribute__((unused)) void *self) {}

static void trait_method(__attribute__((unused)) void *self, char *s) {
    printf("Printing from C: `%s`\n", s);
    free(s);
}

int main() {
    MyTraitStruct *ts = NULL;
    my_trait_struct_new(NULL, trait_destroy, trait_method, &ts);
    use_trait(ts);
    my_trait_struct_destroy(ts);

    MyTraitStruct *ts_from_lib = NULL;
    impl_my_trait_new(42, &ts_from_lib);
    use_trait(ts_from_lib);
    my_trait_struct_destroy(ts_from_lib);
}
