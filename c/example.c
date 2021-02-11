#include <stdio.h>

#include "bindings.h"

// static uint32_t custom_do_something(__attribute__ ((unused)) const void* this_, uint32_t val) {
//     return val + 4242;
// }

int main() {
    char *res = hello_static("World!");
    printf("Result: '%s'\n", res);
    free(res);

    struct HelloStruct *s;
    hello_struct_new(&s);
    res = hello_method((const struct HelloStruct*) &s, "StructWorld!");
    printf("Result: '%s'\n", res);
    free(res);
    hello_struct_destroy(s);

    test_2("AAAAAAAAAAAAAAHHHHHHHHHH");

    // struct Wallet *wallet;
    // wallet_new("Wallet Name", &wallet);

    // struct TxBuilder *tx_builder;
    // create_tx(wallet, &tx_builder);

    // disable_flag(tx_builder);

    // struct CoinSelection triple_cs = triple_cs_new(1000);
    // coin_selection(tx_builder, triple_cs);

    // struct CoinSelection custom_cs = {
    //     .this_ = NULL,
    //     .fn_do_something = &custom_do_something,
    //     .destroy = NULL,
    // };
    // coin_selection(tx_builder, custom_cs);

    // char *wallet_name = get_wallet_name(tx_builder);
    // printf("Wallet name is: '%s'\n", wallet_name);
    // free(wallet_name);

    // printf("The result is: %d\n", finish(tx_builder));

    // wallet_destroy(wallet);
}
