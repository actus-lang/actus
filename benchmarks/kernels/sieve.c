/* Sieve of Eratosthenes below 1 000 000, three rounds: array indexing. */
#include <stdint.h>
#include <string.h>

static uint64_t count_primes(void) {
    static uint8_t composite[1000000];
    memset(composite, 0, sizeof composite);
    uint64_t count = 0;
    for (uint64_t candidate = 2; candidate < 1000000; candidate++) {
        if (composite[candidate] == 0) {
            count++;
            for (uint64_t multiple = candidate * candidate; multiple < 1000000; multiple += candidate) {
                composite[multiple] = 1;
            }
        }
    }
    return count;
}

int main(void) {
    uint64_t total = 0;
    for (uint64_t round = 0; round < 3; round++) {
        total += count_primes();
    }
    return (int)(total % 256);
}
