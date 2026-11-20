/* Collatz steps for every start below 300 000: branchy u64 loops. */
#include <stdint.h>

static uint64_t steps(uint64_t value) {
    uint64_t count = 0;
    while (value != 1) {
        value = value % 2 == 0 ? value / 2 : value * 3 + 1;
        count++;
    }
    return count;
}

int main(void) {
    uint64_t total = 0;
    for (uint64_t start = 1; start < 300000; start++) {
        total += steps(start);
    }
    return (int)(total % 256);
}
