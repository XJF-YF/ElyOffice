/* JSON numbers use JSON syntax. */

#include <math.h>

#include "ely_shim.h"

static fz_json number_of(fz_context *ctx, float value)
{
	fz_json number = { 0 };
	if (!isfinite(value))
		fz_throw(ctx, FZ_ERROR_FORMAT, "a non-finite JSON number");
	number.type = FZ_JSON_NUMBER;
	number.u.number = value;
	return number;
}

void ely_json_number(fz_context *ctx, fz_output *out, float value)
{
	fz_json number = number_of(ctx, value);
	fz_write_json(ctx, out, &number);
}

void ely_json_rect(fz_context *ctx, fz_output *out, fz_rect rect)
{
	float sides[] = { rect.x0, rect.y0, rect.x1, rect.y1 };
	int i;
	fz_write_byte(ctx, out, '[');
	for (i = 0; i < 4; i++)
	{
		if (i)
			fz_write_byte(ctx, out, ',');
		ely_json_number(ctx, out, sides[i]);
	}
	fz_write_byte(ctx, out, ']');
}

void ely_json_append_rect(fz_context *ctx, fz_buffer *buf, fz_rect rect)
{
	float sides[] = { rect.x0, rect.y0, rect.x1, rect.y1 };
	int i;
	fz_append_byte(ctx, buf, '[');
	for (i = 0; i < 4; i++)
	{
		fz_json number = number_of(ctx, sides[i]);
		if (i)
			fz_append_byte(ctx, buf, ',');
		fz_append_json(ctx, buf, &number);
	}
	fz_append_byte(ctx, buf, ']');
}
