#include <sysio.h>
#include <ucm.h>
#include <stdio.h>
#include <memory.h>
#include "video.h"
#include "graphics.h"

u_int frameDone = 0, frameTick = 0;

int curIcfA = ICF_MAX;
int curIcfB = ICF_MAX;
u_char *fb_black = NULL;
short currentLctA = 0;

void fillBuffer(buffer, data, size) register u_int *buffer;
register u_int data, size;
{
	int i;
	size = size >> 2;
	for (i = 0; i < size; i++)
	{
		*buffer++ = data;
	}
}

void fillVideoBuffer(videoBuffer, data) register u_int *videoBuffer;
u_int data;
{
	fillBuffer(videoBuffer, data, VBUFFER_SIZE);
}
#define PIXEL_CORD(x, y) ((x) + (y) * SCREEN_WIDTH)

void setPixel(unsigned char *fb, int x, int y, int color)
{
	fb[PIXEL_CORD(x, y)] = color;
}

nextframe_offset to_delete[2][8];
short to_delete_index[2] = 0;

short CountUsedPCLs();

extern unsigned short current_seqnum;
extern unsigned short min_full_cnt;
extern unsigned short max_full_cnt;

void VBlankOccured()
{
	static short printcnt = 0;

	/* Delay start by 2 frames to sync with audio */
	static long phase_accu = -0x10000 * 2;

	/* We start in an underflow */
	static short already_told_underflow = 1;

	/*
	 * Perform frame rate conversion
	 * Display rate is 50 HZ. Video is 29.97 Hz.
	 * 39282 ~= 2^16 * 29.97 / 50
	 * 32735 ~= 2^16 * 29.97 / 60
	 */
	if (videoMode == 0)
		phase_accu += 39282; /* 29.97 Hz -> 50 Hz */
	else
		phase_accu += 32735; /* 29.97 Hz -> 60 Hz */

	if (!nextframe_valid && phase_accu >= 0x10000)
	{
		if (!already_told_underflow)
			printf("Display underflow\n");
		already_told_underflow = 1;
	}

	if (nextframe_valid && phase_accu >= 0x10000)
	{
		short i;
		phase_accu -= 0x10000;
		already_told_underflow = 0;

		if (nextframe_offsets[0].line != 0)
		{
			printf("Program flow error 3\n");
			exit(1);
		}

		/* Uses ~2% of CPU load. First line cannot be set via lct_a_hwbuf[]. Must be set via dc_wrli() */
		dc_wrli(videoPath, lctA[currentLctA], nextframe_offsets[0].line, 0, cp_dadr((int)nextframe_offsets[0].adr));

		/* Remove video address jumps from the last frame */
		for (i = 0; i < to_delete_index[currentLctA]; i++)
		{
			lct_a_hwbuf[currentLctA][to_delete[currentLctA][i].line * 16] = cp_nop();
		}

		to_delete_index[currentLctA] = 0;
		for (i = 1; i < nextframe_index; i++)
		{
			to_delete[currentLctA][to_delete_index[currentLctA]].line = nextframe_offsets[i].line;
			to_delete_index[currentLctA]++;
			lct_a_hwbuf[currentLctA][nextframe_offsets[i].line * 16] = cp_dadr((int)nextframe_offsets[i].adr);
		}

		/* Uses ~2% of CPU load. Switches LCTs for next frame */
		dc_flnk(videoPath, fctA, lctA[currentLctA], 0);

		currentLctA = (currentLctA + 1) & 1;

		nextframe_index = 0;
		nextframe_valid = 0;
		printcnt++;
		if ((printcnt & 0x3) == 0)
		{
			printf("%d %d %d\n", current_seqnum, CountUsedPCLs(), unused_header);
			unused_header = 0;
		}
	}
}

void createVideoBuffers()
{
	int x;

	fb_black = (u_char *)srqcmem(VBUFFER_SIZE, VIDEO1);
	/* fb_committed = (u_char *)srqcmem(VBUFFER_SIZE, VIDEO1); */
	fillVideoBuffer(fb_black, 0);

	dc_wrli(videoPath, lctA[0], 0, 0, cp_dadr((int)fb_black + pixelStart));
	dc_wrli(videoPath, lctA[0], 0, 6, cp_icf(PA, ICF_MAX));

	dc_wrli(videoPath, lctA[1], 0, 0, cp_dadr((int)fb_black + pixelStart));
	dc_wrli(videoPath, lctA[1], 0, 6, cp_icf(PA, ICF_MAX));

	/* Place SIG at start of frame */
	dc_wrli(videoPath, lctB, 2 * 2, 7, cp_sig());
}

void clearRect(videoBuffer, x, y, width, height, color)
	u_char *videoBuffer;
u_short x, y, width, height;
u_char color;
{
	register u_int value = (color << 24) | (color << 16) | (color << 8) | color;
	register u_int *dst = (u_int *)(videoBuffer + y * SCREEN_WIDTH + x);
	register u_short h, w;

	width >>= 2;

	for (h = 0; h < height; h++)
	{
		for (w = 0; w < width; w++)
			*dst++ = value;
		dst += (SCREEN_WIDTH >> 2) - width;
	}
}

void initGraphics()
{
	createVideoBuffers();
}
