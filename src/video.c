#include <strings.h>
#include <csd.h>
#include <sysio.h>
#include <ucm.h>
#include <stdio.h>
#include <memory.h>
#include "video.h"

int videoPath;
int fctA, fctB, lctB;

int lctA[2];

u_int fctBuffer[FCT_SIZE];
u_int lineSkip;
u_int pixelStart;

u_int *lct_a_hwbuf[2];

/*
 * The Interactive Engineer
 * Volume 5, Number 5, July/August 1996
 * Direct Line Control Table access
 * by Luc Rooijakkers, SPC Vision
 */
u_int *lctAddress(plane, lctid)
int plane;
int lctid;
{
	int lctDummy, lnkInstr;
	/* create a dummy LCT in the same plane */
	lctDummy = dc_crlct(videoPath, plane, 2, 0);
	/* link this dummy LCT to line 1 of our parameter LCT */
	dc_llnk(videoPath, lctDummy, 1, lctid, 1);
	/* read the link instruction written by dc_llnk */
	lnkInstr = dc_rdli(videoPath, lctDummy, 1, 7);
	/* delete the dummy LCT */
	dc_dllct(videoPath, lctDummy);
	/* the address is in the lower 24 bits of the link instr */
	return (u_int *)(lnkInstr & 0x00ffffff);
}

int initFCT(plane, size)
int plane;
int size;
{
	int fct = dc_crfct(videoPath, plane, size, 0);
	return fct;
}

int initLCT(plane, size)
int plane;
int size;
{
	int lct = dc_crlct(videoPath, plane, size, 0);
	dc_nop(videoPath, lct, 0, 0, size, 8); /* Fill all lines and cols of LCT with NOP instructions */
	return lct;
}

#define cl_white(i) cp_clut(i, 255, 255, 255)
#define cl_blue(i) cp_clut(i, 11, 94, 216)
#define cl_red(i) cp_clut(i, 255, 32, 42)
#define cl_green(i) cp_clut(i, 32, 255, 32)
#define cl_dgray(i) cp_clut(i, 38, 43, 68)
#define cl_lgray(i) cp_clut(i, 192, 203, 220)
#define cl_black(i) cp_clut(i, 0, 0, 0)

void setupPlaneA()
{
	int i = 0;
	fctA = initFCT(PA, FCT_SIZE);
	lctA[0] = initLCT(PA, LCT_SIZE);
	lctA[1] = initLCT(PA, LCT_SIZE);
	lct_a_hwbuf[0] = lctAddress(PA, lctA[0]);
	lct_a_hwbuf[1] = lctAddress(PA, lctA[1]);

	dc_flnk(videoPath, fctA, lctA[0], 0);

	fctBuffer[i++] = cp_icm(ICM_CLUT7, ICM_OFF, NM_1, EV_ON, CS_A); /* Use CLUT7 for plane A and B, 1 Matte, External Video On */
	fctBuffer[i++] = cp_tci(MIX_OFF, TR_OFF, TR_OFF);				/* Transparancy Color Key for Plane A and B */
	fctBuffer[i++] = cp_po(PR_AB);									/* Plane A in front of B */
	fctBuffer[i++] = cp_bkcol(BK_BLACK, BK_LOW);					/* Backdrop Low Intensity Black */
	fctBuffer[i++] = cp_tcol(PA, 0, 0, 0);							/* Set transparancy color to black: rgb(0,0,0) */
	fctBuffer[i++] = cp_mcol(PA, 0, 0, 0);							/* Set mask color to black: rgb(0,0,0) */
	fctBuffer[i++] = cp_yuv(PA, 16, 128, 128);						/* Set DYUV start value */
	fctBuffer[i++] = cp_phld(PA, PH_OFF, 1);						/* Set Mosaic (pixel_hold) off, size = 1 */
	fctBuffer[i++] = cp_icf(PA, ICF_MAX);							/* Min Image Contributing Factor */
	fctBuffer[i++] = cp_matte(0, MO_END, MF_MF0, ICF_MAX, 0);
	fctBuffer[i++] = cp_dprm(RMS_RL, PRF_X2, BP_NORMAL); /* Reload Display Parameters */

	fctBuffer[i++] = cp_cbnk(0);
	fctBuffer[i++] = cl_black(0);
	fctBuffer[i++] = cl_white(1);
	fctBuffer[i++] = cl_white(2);
	fctBuffer[i++] = cl_green(3);

	fctBuffer[i++] = cp_cbnk(3);
	fctBuffer[i++] = cl_black(0);
	fctBuffer[i++] = cl_white(1);
	fctBuffer[i++] = cl_white(2);
	fctBuffer[i++] = cl_green(3);

	dc_wrfct(videoPath, fctA, 0, i, fctBuffer);
}

void setupPlaneB()
{
	int i = 0;
	fctB = initFCT(PB, FCT_SIZE);
	lctB = initLCT(PB, LCT_SIZE);
	dc_flnk(videoPath, fctB, lctB, 0);

	fctBuffer[i++] = cp_nop();
	fctBuffer[i++] = cp_nop();
	fctBuffer[i++] = cp_nop();
	fctBuffer[i++] = cp_nop();
	fctBuffer[i++] = cp_tcol(PB, 0, 0, 0);	   /* Set transparancy color to black: rgb(0,0,0) */
	fctBuffer[i++] = cp_mcol(PB, 0, 0, 0);	   /* Set mask color to black: rgb(0,0,0) */
	fctBuffer[i++] = cp_yuv(PB, 16, 128, 128); /* Set DYUV start value */
	fctBuffer[i++] = cp_phld(PB, PH_OFF, 1);   /* Set Mosaic (pixel_hold) off, size = 1 */
	fctBuffer[i++] = cp_icf(PB, ICF_MAX);	   /* Min Image Contributing Factor */
	fctBuffer[i++] = cp_nop();
	fctBuffer[i++] = cp_dprm(RMS_NORMAL, PRF_X2, BP_NORMAL); /* Reload Display Parameters */

	fctBuffer[i++] = cp_cbnk(0);
	fctBuffer[i++] = cl_black(0);
	fctBuffer[i++] = cl_white(1);
	fctBuffer[i++] = cl_white(2);
	fctBuffer[i++] = cl_green(3);

	fctBuffer[i++] = cp_cbnk(3);
	fctBuffer[i++] = cl_black(0);
	fctBuffer[i++] = cl_white(1);
	fctBuffer[i++] = cl_white(2);
	fctBuffer[i++] = cl_green(3);

	dc_wrfct(videoPath, fctB, 0, i, fctBuffer);
}

void initVideo()
{
	char *devName = csd_devname(DT_VIDEO, 1); /* Get Video Device Name */
	char *devParam;
	int videoMode;

	videoPath = open(devName, UPDAT_); /* Open Video Device */
	devParam = csd_devparam(devName);

	videoMode = findstr(1, devParam, "LI=\"625\":") ? 0 : (findstr(1, devParam, "TV") ? 1 : 2); /* First parameter is first character to start searching at; 1-based, not 0-based! */
	printf("Video: %s %d\n", devParam, videoMode);
	free(devName); /* Release memory */
	free(devParam);

	/* Setup Video */
	if (videoMode == 0)
	{ /* PAL - 384x280 */
		dc_setcmp(videoPath, 0);
		lineSkip = 0;
		pixelStart = 0;
	}
	else if (videoMode == 1)
	{ /* NTSC TV - 384x240 */
		dc_setcmp(videoPath, 0);
		lineSkip = 20;
		pixelStart = lineSkip * SCREEN_WIDTH;
	}
	else
	{ /* NTSC Monitor - 360x240 */
		dc_setcmp(videoPath, 1);
		lineSkip = 20;
		pixelStart = 20 * SCREEN_WIDTH;
	}

	dc_intl(videoPath, 0); /* No interlace */

	gc_hide(videoPath); /* Hide the Graphics Cursor */

	setupPlaneA();
	setupPlaneB();
	dc_exec(videoPath, fctA, fctB);
}

void closeVideo()
{
	dc_dllct(videoPath, lctA);
	dc_dllct(videoPath, lctB);
	dc_dlfct(videoPath, fctA);
	dc_dlfct(videoPath, fctB);
	close(videoPath); /* Close Video Device */
}
