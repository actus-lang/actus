/* Recursive Fibonacci of 30: call overhead and return values. */
#include <stdint.h>

static uint64_t fib(uint64_t n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }

int main(void) { return (int)(fib(30) % 256); }
