struct error_info {
	unsigned short code12;	/* 0x0302 looks better than 0x03,0x02 */
	unsigned short size;
};

/*
 * There are 700+ entries in this table. To save space, we don't store
 * (code, pointer) pairs, which would make sizeof(struct
 * error_info)==16 on 64 bits. Rather, the second element just stores
 * the size (including \0) of the corresponding string, and we use the
 * sum of these to get the appropriate offset into additional_text
 * defined below. This approach saves 12 bytes per entry.
 */
static const struct error_info additional[] =
{
#define SENSE_CODE(c, s) {c, sizeof(s)},
#include "sense_codes.h"
#undef SENSE_CODE
};

static const char *additional_text =
#define SENSE_CODE(c, s) s "\0"
#include "sense_codes.h"
#undef SENSE_CODE
	;


void f() {
    &additional;
    &additional_text;
}
