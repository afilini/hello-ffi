#include <stdio.h>

#include "bindings.h"

static uint32_t custom_do_something(__attribute__ ((unused)) const void* this_, uint32_t val) {
    return val + 4242;
}

int main() {
    struct Wallet *wallet;
    wallet_new("Wallet Name", &wallet);

    struct TxBuilder *tx_builder;
    create_tx(wallet, &tx_builder);

    disable_flag(tx_builder);

    struct CoinSelection triple_cs = triple_cs_new(1000);
    coin_selection(tx_builder, triple_cs);

    struct CoinSelection custom_cs = {
        .this_ = NULL,
        .fn_do_something = &custom_do_something,
        .destroy = NULL,
    };
    coin_selection(tx_builder, custom_cs);

    printf("The result is: %d", finish(tx_builder));
}
