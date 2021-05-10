/*
 * libkmipclient - KMIP client library
 *
 * Copyright IBM Corp. 2021
 *
 * s390-tools is free software; you can redistribute it and/or modify
 * it under the terms of the MIT license. See LICENSE for details.
 */
#ifndef UTILS_H
#define UTILS_H

#include <stdint.h>
#include <stdbool.h>

#include <openssl/bn.h>

#include "kmip.h"

#define KMIP_BIG_INTEGER_BLOCK_LENGTH	8

#define KMIP_ISO8601_TIMESTAMP_UTC	"%FT%TZ"
#define KMIP_ISO8601_TIMESTAMP_TZ	"%FT%T%z"
#define KMIP_ISO8601_TIMESTAMP		"%FT%T"

#define kmip_debug(debug, fmt...)					\
		do {							\
			if (debug)					\
				kmip_print_debug(__func__, fmt);	\
		} while (0)

void kmip_print_debug(const char *func, const char *fmt, ...);
void kmip_print_dump(const char *func, unsigned char *data, size_t size,
		     unsigned int indent);

int kmip_parse_decimal_int(const char *str, int64_t *val);
int kmip_parse_decimal_uint(const char *str, uint64_t *val);

int kmip_parse_hex_int(const char *str, int64_t *val);
int kmip_parse_hex(const char *str, bool has_prefix, unsigned char **val,
		   uint32_t *length);
int kmip_format_hex(const unsigned char *val, uint32_t length, bool prefix,
		    char **str);

int kmip_parse_bignum(const char *str, bool has_prefix, BIGNUM **bn);
int kmip_format_bignum(const BIGNUM *bn, bool prefix, char **str);
int kmip_decode_bignum(const unsigned char *data, uint32_t length, BIGNUM **bn);
uint32_t kmip_encode_bignum_length(const BIGNUM *bn);
int kmip_encode_bignum(const BIGNUM *bn, unsigned char *data, uint32_t length);

int kmip_parse_timestamp(const char *str, int64_t *val);

int kmip_parse_mask(enum kmip_tag tag, const char *str, char separator,
		    int64_t *val);
int kmip_format_mask(enum kmip_tag tag, int32_t value, char separator,
		     char **str);

void kmip_node_dump(struct kmip_node *node, bool debug);

enum kmip_tag kmip_find_v1_attribute_name_tag(struct kmip_node *parent);

#endif
