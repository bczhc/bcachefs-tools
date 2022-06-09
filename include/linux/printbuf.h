/* SPDX-License-Identifier: LGPL-2.1+ */
/* Copyright (C) 2022 Kent Overstreet */

#ifndef _LINUX_PRINTBUF_H
#define _LINUX_PRINTBUF_H

/*
 * Printbufs: Simple strings for printing to, with optional heap allocation
 *
 * This code has provisions for use in userspace, to aid in making other code
 * portable between kernelspace and userspace.
 *
 * Basic example:
 *   struct printbuf buf = PRINTBUF;
 *
 *   prt_printf(&buf, "foo=");
 *   foo_to_text(&buf, foo);
 *   printk("%s", buf.buf);
 *   printbuf_exit(&buf);
 *
 * Or
 *   struct printbuf buf = PRINTBUF_EXTERN(char_buf, char_buf_size)
 *
 * We can now write pretty printers instead of writing code that dumps
 * everything to the kernel log buffer, and then those pretty-printers can be
 * used by other code that outputs to kernel log, sysfs, debugfs, etc.
 *
 * Memory allocation: Outputing to a printbuf may allocate memory. This
 * allocation is done with GFP_KERNEL, by default: use the newer
 * memalloc_*_(save|restore) functions as needed.
 *
 * Since no equivalent yet exists for GFP_ATOMIC/GFP_NOWAIT, memory allocations
 * will be done with GFP_NOWAIT if printbuf->atomic is nonzero.
 *
 * Memory allocation failures: We don't return errors directly, because on
 * memory allocation failure we usually don't want to bail out and unwind - we
 * want to print what we've got, on a best-effort basis. But code that does want
 * to return -ENOMEM may check printbuf.allocation_failure.
 *
 * Indenting, tabstops:
 *
 * To aid is writing multi-line pretty printers spread across multiple
 * functions, printbufs track the current indent level.
 *
 * printbuf_indent_push() and printbuf_indent_pop() increase and decrease the current indent
 * level, respectively.
 *
 * To use tabstops, set printbuf->tabstops[]; they are in units of spaces, from
 * start of line. Once set, prt_tab() will output spaces up to the next tabstop.
 * prt_tab_rjust() will also advance the current line of text up to the next
 * tabstop, but it does so by shifting text since the previous tabstop up to the
 * next tabstop - right justifying it.
 *
 * Make sure you use prt_newline() instead of \n in the format string for indent
 * level and tabstops to work corretly.
 *
 * Output units: printbuf->units exists to tell pretty-printers how to output
 * numbers: a raw value (e.g. directly from a superblock field), as bytes, or as
 * human readable bytes. prt_units() obeys it.
 */

#include <linux/kernel.h>
#include <linux/string.h>

enum printbuf_si {
	PRINTBUF_UNITS_2,	/* use binary powers of 2^10 */
	PRINTBUF_UNITS_10,	/* use powers of 10^3 (standard SI) */
};

struct printbuf {
	char			*buf;
	unsigned		size;
	unsigned		pos;
	unsigned		last_newline;
	unsigned		last_field;
	unsigned		indent;
	/*
	 * If nonzero, allocations will be done with GFP_ATOMIC:
	 */
	u8			atomic;
	bool			allocation_failure:1;
	bool			heap_allocated:1;
	enum printbuf_si	si_units:1;
	bool			human_readable_units:1;
	u8			tabstop;
	u8			tabstops[4];
};

int printbuf_make_room(struct printbuf *, unsigned);
const char *printbuf_str(const struct printbuf *);
void printbuf_exit(struct printbuf *);

void prt_newline(struct printbuf *);
void printbuf_indent_add(struct printbuf *, unsigned);
void printbuf_indent_sub(struct printbuf *, unsigned);
void prt_tab(struct printbuf *);
void prt_tab_rjust(struct printbuf *);
void prt_human_readable_u64(struct printbuf *, u64);
void prt_human_readable_s64(struct printbuf *, s64);
void prt_units_u64(struct printbuf *, u64);
void prt_units_s64(struct printbuf *, s64);

/* Initializer for a heap allocated printbuf: */
#define PRINTBUF ((struct printbuf) { .heap_allocated = true })

/* Initializer a printbuf that points to an external buffer: */
#define PRINTBUF_EXTERN(_buf, _size)			\
((struct printbuf) {					\
	.buf	= _buf,					\
	.size	= _size,				\
})

/*
 * Returns size remaining of output buffer:
 */
static inline unsigned printbuf_remaining_size(struct printbuf *out)
{
	return out->pos < out->size ? out->size - out->pos : 0;
}

/*
 * Returns number of characters we can print to the output buffer - i.e.
 * excluding the terminating nul:
 */
static inline unsigned printbuf_remaining(struct printbuf *out)
{
	return out->pos < out->size ? out->size - out->pos - 1 : 0;
}

static inline unsigned printbuf_written(struct printbuf *out)
{
	return min(out->pos, out->size);
}

/*
 * Returns true if output was truncated:
 */
static inline bool printbuf_overflowed(struct printbuf *out)
{
	return out->pos >= out->size;
}

static inline void printbuf_nul_terminate(struct printbuf *out)
{
	printbuf_make_room(out, 1);

	if (out->pos < out->size)
		out->buf[out->pos] = 0;
	else if (out->size)
		out->buf[out->size - 1] = 0;
}

static inline void __prt_chars_reserved(struct printbuf *out, char c, unsigned n)
{
	memset(out->buf + out->pos,
	       c,
	       min(n, printbuf_remaining(out)));
	out->pos += n;
}

static inline void prt_chars(struct printbuf *out, char c, unsigned n)
{
	printbuf_make_room(out, n);
	__prt_chars_reserved(out, c, n);
	printbuf_nul_terminate(out);
}

/* Doesn't call printbuf_make_room(), doesn't nul terminate: */
static inline void __prt_char_reserved(struct printbuf *out, char c)
{
	if (printbuf_remaining(out))
		out->buf[out->pos] = c;
	out->pos++;
}

/* Doesn't nul terminate: */
static inline void __prt_char(struct printbuf *out, char c)
{
	printbuf_make_room(out, 1);
	__prt_char_reserved(out, c);
}

static inline void prt_char(struct printbuf *out, char c)
{
	__prt_char(out, c);
	printbuf_nul_terminate(out);
}

static inline void prt_bytes(struct printbuf *out, const void *b, unsigned n)
{
	printbuf_make_room(out, n);

	memcpy(out->buf + out->pos,
	       b,
	       min(n, printbuf_remaining(out)));
	out->pos += n;
	printbuf_nul_terminate(out);
}

static inline void prt_str(struct printbuf *out, const char *str)
{
	prt_bytes(out, str, strlen(str));
}

static inline void prt_hex_byte(struct printbuf *out, u8 byte)
{
	printbuf_make_room(out, 2);
	__prt_char_reserved(out, hex_asc_hi(byte));
	__prt_char_reserved(out, hex_asc_lo(byte));
	printbuf_nul_terminate(out);
}

static inline void prt_hex_byte_upper(struct printbuf *out, u8 byte)
{
	printbuf_make_room(out, 2);
	__prt_char_reserved(out, hex_asc_upper_hi(byte));
	__prt_char_reserved(out, hex_asc_upper_lo(byte));
	printbuf_nul_terminate(out);
}

/**
 * printbuf_reset - re-use a printbuf without freeing and re-initializing it:
 */
static inline void printbuf_reset(struct printbuf *buf)
{
	buf->pos		= 0;
	buf->allocation_failure	= 0;
}

/**
 * printbuf_atomic_inc - mark as entering an atomic section
 */
static inline void printbuf_atomic_inc(struct printbuf *buf)
{
	buf->atomic++;
}

/**
 * printbuf_atomic_inc - mark as leaving an atomic section
 */
static inline void printbuf_atomic_dec(struct printbuf *buf)
{
	buf->atomic--;
}

#endif /* _LINUX_PRINTBUF_H */
