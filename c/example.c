#include <assert.h>
#include <stdio.h>

#include "bindings.h"

#define BDK_BYTE_ARR(arr, size) (Arr_u8){ arr, size }

int main() {
    const uint8_t b[] = {0x88, 0xac};

    Script *s = NULL;
    script_new(BDK_BYTE_ARR(b, sizeof(b)), &s);

    char *s_hex = script_to_hex(s);
    printf("Script hex: %s\n", s_hex);
    free(s_hex);

    char *s_asm = script_asm(s);
    printf("Script asm: %s\n", s_asm);
    free(s_asm);

    return 0;
}
