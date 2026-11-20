/* xorshift32 over 20 000 000 steps: shifts, xor and a bounded sum. */
#include <stdint.h>

int main(void) {
    uint32_t state = 2463534242u;
    uint64_t sum = 0;
    for (uint64_t step = 0; step < 20000000; step++) {
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        sum += state & 255;
    }
    return (int)(sum % 256);
}
