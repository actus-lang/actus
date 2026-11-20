/* A struct built and borrowed by a function in each of 20 000 000 iterations. */
#include <stdint.h>

struct point {
    int64_t x;
    int64_t y;
};

static int64_t sum(const struct point *self) { return self->x + self->y; }

int main(void) {
    int64_t total = 0;
    for (int64_t i = 0; i != 20000000; i++) {
        struct point p = {i % 7, 1};
        total += sum(&p);
    }
    return (int)(total % 100);
}
