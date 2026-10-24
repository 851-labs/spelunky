/* Does an x86 emulator model the x87 FPU? Prints, for each precision-control setting, a sum and a
 * product whose double result depends on the mantissa width the FPU rounds to (see precision.gml),
 * and a few transcendental instructions whose results on Intel hardware are known.
 * Build: gcc -O0 -static -o x87 x87.c (x86_64 or i386). */
#include <stdint.h>
#include <stdio.h>
#include <string.h>

static uint64_t bits(double d) {
    uint64_t u;
    memcpy(&u, &d, 8);
    return u;
}

static void set_pc(unsigned pc) {
    unsigned short cw;
    __asm__ volatile("fnstcw %0" : "=m"(cw));
    cw = (unsigned short)((cw & ~0x300u) | (pc << 8));
    __asm__ volatile("fldcw %0" : : "m"(cw));
}

static double add(double a, double b) {
    double r;
    __asm__ volatile("fldl %1\n\tfaddl %2\n\tfstpl %0" : "=m"(r) : "m"(a), "m"(b));
    return r;
}

static double mul(double a, double b) {
    double r;
    __asm__ volatile("fldl %1\n\tfmull %2\n\tfstpl %0" : "=m"(r) : "m"(a), "m"(b));
    return r;
}

static double op1(int which, double a) {
    double r;
    switch (which) {
    case 0: __asm__ volatile("fldl %1\n\tfsin\n\tfstpl %0" : "=m"(r) : "m"(a)); break;
    case 1: __asm__ volatile("fldl %1\n\tfcos\n\tfstpl %0" : "=m"(r) : "m"(a)); break;
    case 2: __asm__ volatile("fldl %1\n\tfld1\n\tfpatan\n\tfstpl %0" : "=m"(r) : "m"(a)); break;
    default: __asm__ volatile("fldl %1\n\tfsqrt\n\tfstpl %0" : "=m"(r) : "m"(a)); break;
    }
    return r;
}

int main(void) {
    const double two_m53 = 1.1102230246251565e-16, two_m64 = 5.421010862427522e-20;
    const double a = 1 + 1.4901161193847656e-08;                        /* 1 + 2^-26 */
    const double b = 1 + 7.450580596923828e-09 + 4.440892098500626e-16; /* 1 + 2^-27 + 2^-51 */
    static const unsigned pcs[] = {0, 2, 3};
    static const char *names[] = {"24", "53", "64"};
    for (int i = 0; i < 3; i++) {
        set_pc(pcs[i]);
        printf("pc%s add %016llx mul %016llx\n", names[i], (unsigned long long)bits(add(1, two_m53 + two_m64)),
               (unsigned long long)bits(mul(a, b)));
    }
    set_pc(2);
    /* Intel's fsin(double(pi)) is 1.2246063538223773e-16 (66-bit pi); the true sine is 1.2246467991473532e-16. */
    printf("fsin(pi) %.17g %016llx\n", op1(0, 3.141592653589793), (unsigned long long)bits(op1(0, 3.141592653589793)));
    printf("fsin(1e22) %016llx\n", (unsigned long long)bits(op1(0, 1e22)));
    printf("fcos(1e6) %016llx\n", (unsigned long long)bits(op1(1, 1e6)));
    printf("fsin(0.37) %016llx\n", (unsigned long long)bits(op1(0, 0.37)));
    printf("fpatan(0.37) %016llx\n", (unsigned long long)bits(op1(2, 0.37)));
    printf("fsqrt(2) %016llx\n", (unsigned long long)bits(op1(3, 2)));
    return 0;
}
