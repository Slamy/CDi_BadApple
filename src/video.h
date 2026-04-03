#ifndef __VIDEO_H__
#define __VIDEO_H__

#define FCT_SIZE 160
#define LCT_SIZE 280 * 2 + 2

#define FCT_PAL_START 12

#define SCREEN_WIDTH 384
#define SCREEN_HEIGHT 280
#define SCREEN_SIZE (SCREEN_WIDTH * SCREEN_HEIGHT)
#define VBUFFER_SIZE (SCREEN_HEIGHT * 200) /* First full block after SCREEN_WIDTH * SCREEN_HEIGHT */

extern int videoPath;
extern int fctA, fctB, lctB;
extern u_int fctBuffer[FCT_SIZE];
extern u_int pixelStart;
extern u_int lineSkip;
extern int lctA[2];
extern u_int *lct_a_hwbuf[2];
extern int videoMode;

#define SIG_BLANK 0x0100

#endif