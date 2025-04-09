#include "inc.h"

	.text
	.code16

SYM_FUNC_START_NOALIGN(protected_mode_jump)
	movl	%edx, %esi		# Pointer to boot_params table
	addl	%ebx, 2f
