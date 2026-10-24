/* The x87's own answers, to compare with GM8's: reads lines "OP X [Y]" (X and Y as IEEE-754 bits in hex)
 * and prints the double each instruction gives, loading doubles, with the precision control at 53 bits as
 * GM8 sets it, and storing the result to a double. OP is sin (fsin), cos (fcos), tan (fptan), atan
 * (fpatan of X / 1) or atan2 (fpatan of Y = X, X = Y: arctan2(X, Y)).
 * Build: cc -O0 -o raw raw.c, on x86_64 or i386. */
#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static double real(uint64_t u) {
    double d;
    memcpy(&d, &u, 8);
    return d;
}

static uint64_t bits(double d) {
    uint64_t u;
    memcpy(&u, &d, 8);
    return u;
}

int main(void) {
    unsigned short cw;
    __asm__ volatile("fnstcw %0" : "=m"(cw));
    cw = (unsigned short)((cw & ~0x300u) | 0x200u);
    __asm__ volatile("fldcw %0" : : "m"(cw));
    char op[16];
    uint64_t a, b = 0;
    char line[128];
    while (fgets(line, sizeof line, stdin)) {
        int n = sscanf(line, "%15s %" SCNx64 " %" SCNx64, op, &a, &b);
        if (n < 2) continue;
        double x = real(a), y = real(b), r;
        if (!strcmp(op, "sin")) __asm__ volatile("fldl %1\n\tfsin\n\tfstpl %0" : "=m"(r) : "m"(x));
        else if (!strcmp(op, "cos")) __asm__ volatile("fldl %1\n\tfcos\n\tfstpl %0" : "=m"(r) : "m"(x));
        else if (!strcmp(op, "tan")) __asm__ volatile("fldl %1\n\tfptan\n\tfstp %%st(0)\n\tfstpl %0" : "=m"(r) : "m"(x));
        else if (!strcmp(op, "atan")) __asm__ volatile("fldl %1\n\tfld1\n\tfpatan\n\tfstpl %0" : "=m"(r) : "m"(x));
        else if (!strcmp(op, "atan2")) __asm__ volatile("fldl %1\n\tfldl %2\n\tfpatan\n\tfstpl %0" : "=m"(r) : "m"(x), "m"(y));
        else continue;
        printf("%s %016" PRIx64 " %016" PRIx64 "\n", op, a, bits(r));
    }
    return 0;
}
